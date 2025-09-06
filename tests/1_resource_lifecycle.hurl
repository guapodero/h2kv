# https://hurl.dev/docs/hurl-file.html

GET https://localhost:{{PORT}}/foz
HTTP/2 404

PUT https://localhost:{{PORT}}/foz
`plaintext`
HTTP/2 201
content-location: /foz

HEAD https://localhost:{{PORT}}/foz
HTTP/2 200
content-type: application/octet-stream
content-length: 9

PUT https://localhost:{{PORT}}/foz
`plaintext updated`
HTTP/2 204
content-location: /foz

GET https://localhost:{{PORT}}/foz
HTTP/2 200
content-type: application/octet-stream
content-length: 17
`plaintext updated`

DELETE https://localhost:{{PORT}}/foz
HTTP/2 204

HEAD https://localhost:{{PORT}}/foz
HTTP/2 404

