# https://hurl.dev/docs/hurl-file.html

# this test is set up by creating a file `temporary_directory/sync_file`
# and using the CLI flag `--sync-dir temporary_directory`

GET https://localhost:{{PORT}}/sync_file
HTTP/2 200
content-type: application/octet-stream
`sync_file contents`

PUT https://localhost:{{PORT}}/sync_file
`sync_file contents updated`
HTTP/2 204

# the contents of the file are verified after all tests are completed
