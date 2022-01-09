local Boundary = "------------------------5e4212df18ad08a3"

local BodyBoundary = "--" .. Boundary
local CRLF = "\r\n"
local ContentDisposition = "Content-Disposition: form-data; name=\"content\""
local FieldValue = "foo"
local LastBoundary = "--" .. Boundary .. "--"

wrk.method = "POST"
wrk.headers["Content-Type"] = "multipart/form-data; boundary=" .. Boundary
wrk.body = BodyBoundary .. CRLF .. ContentDisposition ..CRLF .. CRLF .. FieldValue  .. CRLF .. LastBoundary
