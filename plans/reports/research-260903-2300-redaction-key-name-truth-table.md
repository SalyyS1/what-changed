# Key-name redaction: decision table

Ground truth for the rule under design: `normalize_key` lowercases and drops `_ - . space`; a value is sensitive if the normalized name is in the whole-name list, or ends with `key`, `token` or `secret`.
`expected is_sensitive` is what that rule returns, tagged `hit` (right to redact), `over` (redacts a non-secret), `miss` (leaks a real credential), `neg` (right to keep).
The last column is what the already-written `src-tauri/src/privacy/keys.rs` returns; it splits the *original* name into words and matches the last one, so it disagrees on 16 rows. `same` means the two rules agree.
No row is a real credential value; only names are listed.

## Truth table

### `key` family

| key name | normalized | expected is_sensitive | why | keys.rs |
|---|---|---|---|---|
| `API_KEY` | apikey | true | hit — whole-name `apikey` | same |
| `api-key` | apikey | true | hit — separator dropped | same |
| `Api.Key` | apikey | true | hit — dot dropped, case folded | same |
| `apiKey` | apikey | true | hit — camel hump folded | same |
| `X-Api-Key` | xapikey | true | hit — suffix `key` | same |
| `OPENAI_API_KEY` | openaiapikey | true | hit — suffix `key` | same |
| `AWS_SECRET_ACCESS_KEY` | awssecretaccesskey | true | hit — suffix `key` | same |
| `ACCESS_KEY_ID` | accesskeyid | false | miss — trailing `id` defeats the suffix, AWS key id leaks | true |
| `API_KEY_VALUE` | apikeyvalue | false | miss — trailing `value` defeats the suffix | true |
| `PRIVATE_KEY` | privatekey | true | hit — whole name and suffix | same |
| `private.key` | privatekey | true | hit — same key, dotted spelling | same |
| `ENCRYPTION_KEY` | encryptionkey | true | hit — real material | same |
| `KEY` | key | true | hit — the whole name is the stem | same |
| `KEYS` | keys | false | neg by luck — plural escapes the suffix, a JWKS blob survives | same |
| `KEY_ID` | keyid | false | neg — KMS identifier, not material | true |
| `LICENSE_KEY` | licensekey | true | hit — entitlement secret | same |
| `PRODUCT_KEY` | productkey | true | hit — entitlement secret | same |
| `KEYBOARD_LAYOUT` | keyboardlayout | false | neg — `key` is a prefix, not a suffix | same |
| `KEY_VAULT_NAME` | keyvaultname | false | neg — resource name, not the secret it holds | same |
| `HOSTKEY_ALGORITHM` | hostkeyalgorithm | false | neg — algorithm id such as `ssh-ed25519` | same |
| `API_PORT` | apiport | false | neg — no stem anywhere | same |

### `token`, bearer and authorization

| key name | normalized | expected is_sensitive | why | keys.rs |
|---|---|---|---|---|
| `TOKEN` | token | true | hit — whole name | same |
| `ACCESS_TOKEN` | accesstoken | true | hit — suffix `token` | same |
| `REFRESH_TOKEN` | refreshtoken | true | hit — whole name and suffix | same |
| `X_AUTH_TOKEN` | xauthtoken | true | hit — suffix `token` | same |
| `GITHUB_TOKEN` | githubtoken | true | hit — suffix `token` | same |
| `NGROK_AUTHTOKEN` | ngrokauthtoken | true | hit — the suffix test reaches inside the glued word | false |
| `TOKENIZER_PATH` | tokenizerpath | false | neg — suffix beats a `contains` test here | same |
| `TOKEN_BUCKET_SIZE` | tokenbucketsize | false | neg — rate-limiter config | same |
| `JWT` | jwt | false | miss — bare name, the `eyJ…` value pattern must carry it | same |
| `AUTH_HEADER` | authheader | false | miss — usually holds `Bearer …`, value pattern must carry it | same |
| `BEARER` | bearer | true | hit — whole name | same |
| `BEARER_PREFIX` | bearerprefix | false | neg — holds the literal string `Bearer ` | same |
| `AUTHORIZATION` | authorization | true | hit — whole name | same |
| `authorisation` | authorisation | false | miss — the `-ise` spelling is absent from the list | true |

### cookie and session

| key name | normalized | expected is_sensitive | why | keys.rs |
|---|---|---|---|---|
| `Cookie` | cookie | true | hit — whole name | same |
| `Set-Cookie` | setcookie | false | miss — carries the session credential itself | true |
| `COOKIE_SECURE` | cookiesecure | false | neg — boolean flag, and `secure` is not `secret` | same |
| `SESSION` | session | true | hit — whole name | same |
| `SESSION_ID` | sessionid | false | miss — a session id is bearer-equivalent | true |
| `SESSIONID` | sessionid | false | miss — same value, glued spelling | same |
| `SESSION_SECRET` | sessionsecret | true | hit — suffix `secret` | same |
| `SESSION_TIMEOUT_MS` | sessiontimeoutms | false | neg — duration | same |

### password family

| key name | normalized | expected is_sensitive | why | keys.rs |
|---|---|---|---|---|
| `PASSWORD` | password | true | hit — whole name | same |
| `PASSWD` | passwd | true | hit — whole name | same |
| `db_password` | dbpassword | false | miss — the list is equality-only and `password` is not a suffix stem | true |
| `DB_PASSWD` | dbpasswd | false | miss — same defect | true |
| `MYSQL_ROOT_PASSWORD` | mysqlrootpassword | false | miss — appears in nearly every compose file | true |
| `KEYSTORE_PASSWORD` | keystorepassword | false | miss — unlocks a Java keystore | true |
| `PGPASSWORD` | pgpassword | false | miss — libpq env var, glued so word splitting misses it too | same |
| `PWD` | pwd | true | over — POSIX working directory, not a password | same |
| `OLDPWD` | oldpwd | false | neg — same kind of value as `PWD`, opposite verdict | same |
| `PASSWORD_MIN_LENGTH` | passwordminlength | false | neg — policy number | same |
| `PASSPHRASE` | passphrase | true | hit — whole name | same |
| `GPG_PASSPHRASE` | gpgpassphrase | false | miss — equality-only again | true |

### secret and credential

| key name | normalized | expected is_sensitive | why | keys.rs |
|---|---|---|---|---|
| `SECRET` | secret | true | hit — whole name | same |
| `CLIENT_SECRET` | clientsecret | true | hit — whole name and suffix | same |
| `WEBHOOK_SECRET` | webhooksecret | true | hit — suffix `secret` | same |
| `SECRETS_DIR` | secretsdir | false | neg — directory path | same |
| `CREDENTIAL` | credential | true | hit — whole name | same |
| `AWS_CREDENTIALS` | awscredentials | false | miss — the plural is neither a whole name nor a stem | true |
| `CREDENTIAL_PATH` | credentialpath | false | neg — a path, not the credential | same |
| `DATABASE_URL` | databaseurl | false | neg at key level — the URI userinfo value pattern owns this | same |

### suffix `key` on values that are not credentials

| key name | normalized | expected is_sensitive | why | keys.rs |
|---|---|---|---|---|
| `PUBLIC_KEY` | publickey | true | over — public, but opaque and a stable fingerprint | same |
| `SSH_PUBLIC_KEY` | sshpublickey | true | over — same, and it identifies the machine | same |
| `STRIPE_PUBLISHABLE_KEY` | stripepublishablekey | true | over — publishable by design | same |
| `PRIMARY_KEY` | primarykey | true | over — schema metadata | same |
| `FOREIGN_KEY` | foreignkey | true | over — schema metadata | same |
| `SORT_KEY` | sortkey | true | over — index attribute name | same |
| `PARTITION_KEY` | partitionkey | true | over — routing metadata | same |
| `CACHE_KEY` | cachekey | true | over — often the URL a developer wants to read in the diff | same |
| `IDEMPOTENCY_KEY` | idempotencykey | true | over — request-scoped, enables replay only | same |

### English words that end in a stem

| key name | normalized | expected is_sensitive | why | keys.rs |
|---|---|---|---|---|
| `MONKEY` | monkey | true | over — `-key` with no word boundary before it | false |
| `DONKEY` | donkey | true | over — same | false |
| `TURKEY` | turkey | true | over — same | false |
| `HOTKEY` | hotkey | true | over — real editor config such as `hotkey: Ctrl+K` | false |
| `HOTKEYS` | hotkeys | false | neg by luck — the plural escapes, so the pair is inconsistent | same |
| `USER_ID` | userid | false | neg — control row, no stem | same |

## Recommendation

1. Ship the last-word rule already written in `src-tauri/src/privacy/keys.rs`, not the normalized-suffix rule. It flips 16 rows above in both directions: `db_password`, `DB_PASSWD`, `MYSQL_ROOT_PASSWORD`, `KEYSTORE_PASSWORD`, `GPG_PASSPHRASE`, `Set-Cookie`, `SESSION_ID`, `ACCESS_KEY_ID`, `API_KEY_VALUE`, `AWS_CREDENTIALS` and `authorisation` stop leaking, and `MONKEY`, `DONKEY`, `TURKEY`, `HOTKEY` stop being redacted.
2. Reject both cheaper guards. A separator check on the original name and a deny-list of animals are subsumed by word splitting, and the deny-list has to grow forever: hockey, jockey, whiskey, lackey, malarkey.
3. Close the hole word splitting opens, because it currently leaks real credentials: `NGROK_AUTHTOKEN`, `PGPASSWORD`, `SESSIONID`. After the last-word test fails, also accept a last word that *ends with* `password`, `passwd`, `passphrase`, `token`, `secret` or `credential` — never `key`, the one stem with English collisions — and add `sessionid` as a whole name. Cost is roughly six lines and four tests; no new false positive was found, since no ordinary English word ends in those stems.
4. Keep redacting public keys. This is not a confidentiality call: the value is a long opaque blob nobody reads in a diff, and it fingerprints the machine and account inside an artifact that gets exported into bug reports. Rotation stays visible through the `changed` flag computed before redaction, so no signal is lost and the placeholder must stay the bare `[REDACTED]` with no digest or prefix.
5. Keep `authorisation` in the whole-name list. It names the same header, no non-secret key is spelled that way, and it costs one string.

## Cases where the rule deliberately over-redacts

- `MONKEY`, `DONKEY`, `TURKEY`, `HOTKEY` — the suffix matches with no word boundary before it. The word-splitting rule already avoids these; under the suffix rule they are the price of catching glued names such as `NGROK_AUTHTOKEN`. `HOTKEYS` escapes on the plural, so the suffix rule is not even self-consistent.
- `PUBLIC_KEY`, `SSH_PUBLIC_KEY`, `STRIPE_PUBLISHABLE_KEY` — public by design, redacted on purpose.
- `PRIMARY_KEY`, `FOREIGN_KEY`, `SORT_KEY`, `PARTITION_KEY` — schema and routing metadata, never credentials.
- `CACHE_KEY` — the costliest false positive, because a cache key usually encodes the request a developer is trying to read.
- `IDEMPOTENCY_KEY` — request-scoped and replay-only, accepted.
- `LICENSE_KEY`, `PRODUCT_KEY` — judged correct rather than over-redaction; both gate a paid entitlement.
- `PWD` — over-redacted by the whole-name list, not the suffix rule: on a developer machine it is the working directory. `KEY_ID` is the same class under the word rule, where the trailing qualifier is stripped and `key` remains.
- `KEY` — the whole name is the stem, and a value filed under a bare `key` is material in practice.
