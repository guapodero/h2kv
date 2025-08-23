use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use bytes::{BufMut, Bytes};
use h2::RecvStream;
use h2::server::{self, SendResponse};
use http::{
    Method, Request, Response, StatusCode, Version,
    header::{self, HeaderMap},
};
use tokio::net::{TcpListener, TcpStream};

use crate::Config;
use crate::storage::StorageBackend;

pub async fn listen(config: &Config, db: Arc<impl StorageBackend>) -> Result<()> {
    let listener = TcpListener::bind(format!("127.0.0.1:{}", config.port)).await?;

    log::info!("listening on {:?}", listener.local_addr()?);

    loop {
        if let Ok((socket, _peer_addr)) = listener.accept().await {
            let db = db.clone();
            tokio::spawn(async move {
                if let Err(e) = serve(socket, db).await {
                    log::error!("H2 listener error: {e:?}");
                }
            });
        }
    }
}

async fn serve(socket: TcpStream, db: Arc<impl StorageBackend>) -> Result<()> {
    let mut connection = server::handshake(socket).await?;
    log::trace!("H2 connection opened");

    while let Some(result) = connection.accept().await {
        let (request, respond) = result?;
        let db = db.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_request(request, respond, db).await {
                log::error!("error while handling request: {e}");
            }
        });
    }

    log::trace!("H2 connection closed");
    Ok(())
}

async fn handle_request(
    mut request: Request<RecvStream>,
    mut respond: SendResponse<Bytes>,
    db: Arc<impl StorageBackend>,
) -> Result<()> {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let path = PathBuf::from(uri.path());
    let headers = request.headers().clone();
    let body = request.body_mut();

    let mut response =
        |status: StatusCode, headers: Option<HeaderMap>, body: Option<Bytes>| -> Result<()> {
            let (mut parts, _) = Response::new(()).into_parts();
            parts.version = Version::HTTP_2;
            parts.status = status;
            parts.headers = headers.unwrap_or_default();
            let response = Response::from_parts(parts, ());
            let mut send = respond.send_response(response, false)?;
            send.send_data(body.unwrap_or_default(), true)?;
            Ok(())
        };

    match (method, path, headers) {
        (method @ (Method::HEAD | Method::GET), path, headers) => {
            log::trace!("received {method} {path:?} with {headers:?}");
            match db.get(&path) {
                Ok(Some(data)) => {
                    let mut headers = HeaderMap::new();
                    headers.append(header::CONTENT_LENGTH, data.len().into());
                    match method {
                        Method::HEAD => {
                            response(StatusCode::OK, Some(headers), None)?;
                        }
                        Method::GET => {
                            response(StatusCode::OK, Some(headers), Some(Bytes::from(data)))?;
                        }
                        _ => unreachable!(),
                    };
                }
                Ok(None) => response(StatusCode::NOT_FOUND, None, None)?,
                Err(e) => {
                    log::error!("error reading database {path:?}: {e}");
                    response(StatusCode::SERVICE_UNAVAILABLE, None, None)?;
                }
            }
        }
        (Method::PUT, path, headers) => {
            log::trace!("received PUT {path:?} with {headers:?}");
            let mut buf = vec![];
            while let Some(data) = body.data().await {
                let data = data?;
                let _ = body.flow_control().release_capacity(data.len());
                buf.put(data);
            }
            let key_exists = db.get(&path)?.is_some();
            db.put(&path, &buf[..])?;
            if !key_exists {
                log::trace!("created {path:?} ({} bytes)", buf.len());
                response(StatusCode::CREATED, None, None)?;
            } else {
                log::trace!("updated {path:?} ({} bytes)", buf.len());
                response(StatusCode::NO_CONTENT, None, None)?;
            }
        }
        (Method::DELETE, path, headers) => {
            log::trace!("received DELETE {path:?} with {headers:?}");
            db.delete(&path)?;
            response(StatusCode::NO_CONTENT, None, None)?;
        }
        (method, path, headers) => {
            log::error!("not implemented: {method:?} {path:?} with {headers:?}");
            response(StatusCode::NOT_IMPLEMENTED, None, None)?;
        }
    }

    Ok(())
}
