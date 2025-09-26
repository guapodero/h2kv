# https://hurl.dev/docs/hurl-file.html

# this test is set up by creating a file `temporary_directory/sync_file`
# and using the CLI flag `--sync-dir temporary_directory`

GET https://localhost:{{PORT}}/sync_file.wasm
HTTP/2 200
content-type: application/wasm
`sync_file contents`

PUT https://localhost:{{PORT}}/sync_file.wasm
`sync_file contents updated`
HTTP/2 204

PUT https://localhost:{{PORT}}/new/file2.txt
`file2 contents`
HTTP/2 201

# the contents of updated files are verified after all tests are completed

GET https://localhost:{{PORT}}/ignored_read
HTTP/2 404

PUT https://localhost:{{PORT}}/ignored_write
`ignored`
HTTP/2 201

