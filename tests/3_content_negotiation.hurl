# https://hurl.dev/docs/hurl-file.html

PUT https://localhost:{{PORT}}/foz.json
`{ "type": "json" }`
HTTP/2 201

GET https://localhost:{{PORT}}/foz.json
HTTP/2 200
# content-type is not guessed from extension
content-type: application/octet-stream

PUT https://localhost:{{PORT}}/foz.json
content-type: application/json
`{ "type": "json with content-type" }`
HTTP/2 204

GET https://localhost:{{PORT}}/foz.json
HTTP/2 200
# content-type has been updated with previous PUT
content-type: application/json
`{ "type": "json with content-type" }`

# any extension is accepted
PUT https://localhost:{{PORT}}/foz.jsonmap
# empty resources are allowed
content-type: application/ld+json
HTTP/2 201

GET https://localhost:{{PORT}}/foz.jsonmap
HTTP/2 200
content-type: application/ld+json
[Asserts]
bytes count == 0

# any formatted "type/subtype" content-type is accepted
PUT https://localhost:{{PORT}}/foz.app
content-type: app # not acceptable
`n/a`
HTTP/2 415

PUT https://localhost:{{PORT}}/foz.app
content-type: apple/catipury
`good`
HTTP/2 201

# a file extension is added, if it can be guessed from content-type header
PUT https://localhost:{{PORT}}/cat
content-type: application/xml
`cat xml`
HTTP/2 201
content-location: /cat.xml

GET https://localhost:{{PORT}}/cat.xml
HTTP/2 200
content-type: application/xml
`cat xml`

# the extension is optional if there is only one content-type to serve
GET https://localhost:{{PORT}}/cat
HTTP/2 200
content-type: application/xml
`cat xml`

# if the extension can not be guessed from content-type, an "extensionless"
# representation of the resource is created
PUT https://localhost:{{PORT}}/cat
content-type: application/cat
`cat app`
HTTP/2 201
content-location: /cat

 # extensionless is served by default
GET https://localhost:{{PORT}}/cat
HTTP/2 200
content-type: application/cat
`cat app`

# other representations can be requested using accept header
# https://www.rfc-editor.org/rfc/rfc9110.html#section-12.5.1
GET https://localhost:{{PORT}}/cat
accept: application/xml
HTTP/2 200
content-type: application/xml
`cat xml`
