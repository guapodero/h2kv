USAGE:
  h2kv  --storage-dir STRING [--port i32] [--sync-dir STRING] [--sync-write] [--daemon] [--pidfile STRING] [--log-filename STRING]

  --storage-dir STRING    directory to use for storage engine files
  [--port i32]            listening port for TCP connections, default: 5928
  [--sync-dir STRING]     directory to synchronize with the database and "host" on start and SIGHUP
  [--sync-write]          write to the synchronized directory on exit and SIGHUP
  [--daemon]              fork into background process
  [--pidfile STRING]      PID file, ignored unless --daemon is set
  [--log-filename STRING] file to send daemon log messages, ignored unless --daemon is set


For more information try --help
