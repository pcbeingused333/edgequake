# Lens — Security

Parent: [README](../README.md) · Laws: [LAW-173-3](../01-first-principles.md), [LAW-173-4](../01-first-principles.md), [LAW-173-5](../01-first-principles.md), [LAW-173-7](../01-first-principles.md)

## The URL is a capability

A presigned PUT is a bearer token for one object, one method, one content type, and one checksum, for at most 900 seconds. [Discussion #1197](https://github.com/modelcontextprotocol/modelcontextprotocol/discussions/1197) asks for exactly that bind. [S3 object integrity](https://docs.aws.amazon.com/AmazonS3/latest/userguide/checking-object-integrity.html) is how the checksum is enforced at the bucket when the header is part of the signature ([EC-173-01](../06-edge-cases.md)).

If the signature does not include the checksum, a URL copied out of a transcript can store a different PDF in the tenant. That configuration is not legal here. `signed_url_opts` returning `NotSupported` falls back to relay rather than dropping the header ([EC-173-17](../06-edge-cases.md)).

## When the URL must not be enough

`EDGEQUAKE_MCP_UPLOAD_BIND=bearer` forces the same-origin relay. The HMAC still binds the hash. The MCP bearer is also required. A copied URL without the bearer is `403` ([EC-173-18](../06-edge-cases.md)). This is the October 2026 working-group case where a pre-authorized third-party URL is the wrong data plane ([03](../03-protocol.md)).

The MAC key is `EDGEQUAKE_MCP_UPLOAD_HMAC_KEY` or the JWT secret. The default insecure JWT secret cannot sign. The key is not a tool field ([EC-173-30](../06-edge-cases.md)).

## Authorization before the signature

The server checks the write scope, the profile, the tenant, and the workspace before it mints a URL ([EC-173-15](../06-edge-cases.md), [EC-173-27](../06-edge-cases.md), [EC-173-04](../06-edge-cases.md)). `user_id` is checked when both sides have one ([EC-173-25](../06-edge-cases.md)). The key prefix is not the check. SPEC-172 [LAW-172-5](../../172-object-store/01-first-principles.md) said the same about downloads. 173 does not mint a GET ([EC-173-19](../06-edge-cases.md)).

## Confused deputy

Commit reads the staging key on the row. It does not fetch a URL the model passes ([EC-173-23](../06-edge-cases.md), [LAW-173-1](../01-first-principles.md)). Filename characters never become the key ([EC-173-09](../06-edge-cases.md)).

## Logs

`sig`, the presigned query, and `Authorization` are not tracing fields ([EC-173-14](../06-edge-cases.md)).

## Bucket policy

Unchanged from SPEC-172. The server identity can put, get, and delete on the prefix. The world cannot. The presign is the only anonymous write, and only for the staging key the signature names.
