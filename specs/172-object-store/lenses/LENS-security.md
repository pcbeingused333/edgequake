# Lens — Security

Parent: [README](../README.md) · Laws: [LAW-172-5](../01-first-principles.md), [LAW-172-8](../01-first-principles.md)

## Trust boundary

The API process is the only component that may turn a user request into a byte read. The bucket policy allows the server's identity `GetObject`, `PutObject`, `DeleteObject`, and `ListBucket` on the one prefix. It does not allow `s3:*`, and it does not allow public `GetObject`.

Presigned URLs are out of this spec. They would move authorization into a URL the browser holds. A later spec can add them with a short TTL and the same workspace check before the signature is minted.

## Keys

UUIDs only ([LAW-172-8](../01-first-principles.md)). A filename with `../` or a null byte cannot change the object that is written. T-172-18 feeds `../café.pdf`.

The prefix groups ops data. It is not an access check. T-172-30 calls get with the right object and the wrong workspace and expects no bytes. RLS on `original_object_refs` is the SQL version of that check ([EC-172-09](../05-edge-cases.md)).

## Secrets

Access keys, Azure keys, and service-account JSON are environment or workload identity. They are not columns. Logs print the bucket and the host, never the secret and never URL userinfo ([08](../08-migration-ux.md)).

`EDGEQUAKE_ORIGINAL_STORE_ALLOW_HTTP` defaults false. Production endpoints are `https`. Local MinIO sets it true on purpose.

SSE defaults off at the client so a local MinIO without KMS still works. Bucket-level encryption stays the account owner's choice. `AES256` is the only client override. A typo fails closed ([EC-172-19](../05-edge-cases.md)).

## Delete

The user-facing delete commits before the object delete. A crash leaves an orphan object, which the reaper removes. The inverse (object gone, row still served) is the worse leak of a false 200. [EC-172-08](../05-edge-cases.md) locks the order.

## Mode

`database` mode with refs present exits. That stops a replica with no bucket credentials from serving holes or from falling back to empty `BYTEA` and calling it a success ([EC-172-20](../05-edge-cases.md)).
