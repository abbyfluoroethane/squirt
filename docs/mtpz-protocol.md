# MTPZ authentication: protocol specification

This document says what an MTP initiator (host) must do to authenticate to a
device that needs MTPZ (Zune, Zune HD, Kin, Windows Phone 7). An implementer
must be able to write a working, interoperable handshake from this document
only.

Notation:

- `a || b` is byte concatenation. `X[i..j)` is bytes `i` up to (not including) `j` of `X`, counted from 0.
- `BE16`/`BE32` are big-endian unsigned integers. `LE` is little-endian.
- All hex constants are exact.
- "SHA-1" is FIPS 180 SHA-1. "AES" is FIPS 197 AES-128. "CMAC" is AES-CMAC
  (NIST SP 800-38B, RFC 4493).
- `MGF1(seed, n)` is MGF1 from PKCS #1 v2.1 with SHA-1: the first `n` bytes of
  `SHA1(seed || BE32(0)) || SHA1(seed || BE32(1)) || ...`.

---

## 1. Overview

MTPZ is a mutual-authentication handshake. It runs inside an open MTP session,
before the device accepts writes. The host has one RSA-1024 key pair and a
certificate chain issued for that key (the "application certificate"). The
device has its own certificate chain.

Summary:

1. The host sends its certificate chain and 16 random bytes. It signs them
   with RSA-PSS (SHA-1, no salt).
2. The device returns a 16-byte AES key wrapped with RSA-OAEP (SHA-1) to the
   host's public key. It also returns a payload encrypted with that key in
   AES-128-CBC. The payload holds the device certificates, an echo of the host
   random, the device random, a device signature and a 32-byte value (the "MAC
   seed" in this document).
3. The host decrypts the payload and checks the echoed random. It then uses
   the first 16 bytes of the MAC seed as an AES-CMAC key. It sends a CMAC
   confirmation and calls EnableTrustedFilesOperations with four 32-bit
   parameters that come from another CMAC.

After step 3 the device allows normal MTP object transfers. Later transfers
are not encrypted.

---

## 2. MTP operations used

| Code     | Name                              | Params | Data phase              | Use in the handshake |
|----------|-----------------------------------|--------|-------------------------|----------------------|
| `0x1016` | SetDevicePropValue (standard)     | 1: `0xD406` | host to device (MTP string) | Set Session Initiator Info |
| `0x9216` | EndTrustedAppSession              | 0      | none                    | Reset any earlier handshake state |
| `0x9212` | SendWMDRMPDAppRequest             | 0      | host to device          | Send the application-certificate message, and later the confirmation message |
| `0x9213` | GetWMDRMPDAppResponse             | 0      | device to host          | Get the device's handshake response |
| `0x9214` | EnableTrustedFilesOperations      | 4      | none                    | Finish the handshake |
| `0x9215` | DisableTrustedFilesOperations     | 0 (assumed) | none               | Not used by the reference flow |

Device property `0xD406` is "Session Initiator Info" (MTP string type,
datatype `0xFFFF`).

Every operation must return response code `0x2001` (OK). If one does not,
stop the handshake.

### 2.1 How to detect an MTPZ device

- The DeviceInfo VendorExtensionDesc string has an element whose name is
  `microsoft.com/MTPZ` (example from a Zune HD: `microsoft.com: 1.0;
  microsoft.com/WMDRMPD: 10.1; microsoft.com/WMPPD: 11.1; microsoft.com/AAVT:
  1.0; microsoft.com/WMDRMND: 1.0; microsoft.com/MTPZ: 1.0; audible.com: 1.0;`).
  Elements are separated by `;`. Each element is `name: major.minor`, and it
  can have leading spaces.
- Also (for information only): the Microsoft OS descriptor (string index
  `0xEE`, vendor code `0xFE`) gives the compatible ID `MTPZ`.
- The Zune HD operations list has `0x9212` to `0x9216` (and also `0x9217` to
  `0x921D`, which the handshake does not use).

USB IDs: Zune HD `045e:063e`. Other Zunes `045e:0710`.

---

## 3. Key file (`.mtpz-data`)

The key file is ASCII text with **five lines**. Each line is hex. Lines end
with LF or CRLF; remove both. The order in the file is shown below. (A comment
in the reference source lists a different order. The order below is the
order the source really reads.)

| Line | Content | Hex length (typical) | How to decode | Used for |
|------|---------|----------------------|---------------|----------|
| 1 | RSA public exponent `e` | up to 6 chars (for example `010001`) | Big-endian unsigned integer | RSA key (the raw private operation does not need it) |
| 2 | "Encryption key" | 32 chars, which is 16 bytes | Bytes | **Not used in the handshake.** See note A. |
| 3 | RSA modulus `n` | 256 chars, which is 128 bytes (1024-bit) | Big-endian unsigned integer | RSA operations |
| 4 | RSA private exponent `d` | 256 chars (can be shorter if there are leading zeros) | Big-endian unsigned integer | RSA operations |
| 5 | Application certificate chain | 1258 chars, which is **629 bytes** | Bytes, already plaintext | Copied unchanged into message 1 |

Notes:

- **A.** In the original Windows/Mac Zune software, a 16-byte AES-128 key
  decrypts (block by block, ECB-style, no IV) the embedded certificate blob
  and an embedded Windows `RSA2` private-key blob. The `.mtpz-data` format
  already holds the decrypted results: line 5 is the certificate, and line 4
  is `d`, worked out from that private-key blob. So line 2 is never needed.
  Parse it and ignore it, but require it to be present so the file layout
  stays compatible.
- Lines 3 and 4 are the plain RSA integers in big-endian hex. They are **not**
  the little-endian `RSA2` blob. There are no CRT parameters. To sign or
  decrypt, compute `x^d mod n` directly (or work out p/q if you want CRT; this
  is optional).
- Upper-case and lower-case hex must both be accepted.
- Recommended consistency check (optional): the modulus inside the leaf
  certificate in line 5 (see 3.1) must be the same as line 3.

### 3.1 Certificate chain layout (information only)

You do not need to parse this to authenticate. It is useful for validation
and for the device certificates in the response, which use the same record
format.

| Offset | Size | Meaning |
|--------|------|---------|
| 0 | 1 | Number of certificates (`0x02` in known key files) |
| 1 | ... | That number of certificate records, one after the other |

Certificate record (from a public writeup; the fields marked "unknown" are
not understood):

| Rel. offset | Size | Meaning |
|-------------|------|---------|
| 0 | 4 | BE32 record body length `L` (the bytes after this field) |
| 4 | 2 | BE16, value `0x0001` seen (unknown) |
| 6 | 4 | BE32 inner length |
| 10 | 1 | Unknown byte (<= 5 seen) |
| 11 | 4 | Unknown (zeros seen) |
| 15 | 16 | Unknown / skipped |
| 31 | 2 | BE16 name length `m` |
| 33 | m | ASCII name (for example "Zune Device CA 1" or "Zune Device"; the host leaf has a "Zune Software Leaf ..." name) |
| 33+m | 4 | BE32 RSA public exponent |
| 37+m | 2 | BE16 modulus length (`0x0080`) |
| 39+m | 128 | RSA modulus, big-endian |
| ... | rest of `L` | Unknown. This is possibly the issuer signature. |

In the known 629-byte host chain, the second record is the host's leaf
certificate. Its modulus is the modulus in line 3.

---

## 4. Handshake sequence

Preconditions: the device is opened, GetDeviceInfo is done, and OpenSession
returned OK. The reference does the handshake after it reads the storage IDs.
Any point after OpenSession and before the first write is expected to work.

1. **SetDevicePropValue(`0xD406`)**. The data is an MTP string (a u8 count of
   UTF-16 code units including the terminating NUL, then UTF-16LE code units
   including the NUL). The reference sends an identifying ASCII string. It
   is not known whether the device checks the content (see Uncertainties).
   Use a string such as `"Squirt/MTPZClassDriver"`.
2. **EndTrustedAppSession (`0x9216`)**, with no params and no data. This
   resets the state.
3. Make 16 random bytes `R_h`. Use a CSPRNG. (The reference uses a weak
   time-seeded PRNG. Do not copy that.)
4. Build **Message 1** (Section 5) and send it with **SendWMDRMPDAppRequest
   (`0x9212`)**.
5. **GetWMDRMPDAppResponse (`0x9213`)** returns **Message 2** (Section 6).
6. Process Message 2 (Section 6.2). This gives the 16-byte session key
   `K_s`, the decrypted payload, and the 32-byte MAC seed `S`. Check that the
   echoed random is the same as `R_h`.
7. Build **Message 3** (Section 7) and send it with **SendWMDRMPDAppRequest
   (`0x9212`)**.
8. Compute the four parameters (Section 8) and send
   **EnableTrustedFilesOperations (`0x9214`)** with them.
9. The handshake is done. Continue with the normal MTP operations.

If any step fails, the device is in an unknown handshake state. Do step 2
again before you try again.

---

## 5. Message 1: application-certificate message (host to device)

The total length is **785 bytes** (with the 629-byte chain from the key file).

| Offset | Size | Value | Meaning |
|--------|------|-------|---------|
| 0 | 2 | `02 01` | Message type marker (message 1) |
| 2 | 1 | `01` | Marker |
| 3 | 4 | `00 00 02 75` | BE32 certificate chain length = 629 |
| 7 | 629 | key file line 5 | Certificate chain, unchanged |
| 636 | 2 | `00 10` | BE16 random length = 16 |
| 638 | 16 | `R_h` | Host random |
| 654 | 1 | `01` | Marker |
| 655 | 2 | `00 80` | BE16 signature length = 128 |
| 657 | 128 | `Sig` | RSA signature, big-endian, left-padded with zeros to 128 bytes |

(If a key file has a chain with a different length, the lengths and offsets
move to match. The BE32 at offset 3 must give the real length. Known key
files are all 629 bytes.)

### 5.1 Signature: RSASSA-PSS, SHA-1, salt length 0

This is the standard PKCS #1 v2.1 EMSA-PSS encoding with hash SHA-1, MGF1-SHA-1,
**sLen = 0**, emBits = 1023 (modBits - 1) and emLen = 128. Then comes a raw
RSA private operation. Steps:

1. `T = Message1[2..654)`. That is everything from the `01` marker at offset 2
   up to and including the last byte of `R_h` (652 bytes). The two leading
   bytes `02 01` are **not** signed.
2. `mHash = SHA1(T)` (20 bytes).
3. `H = SHA1( 00×8 || mHash )` (8 zero bytes, then mHash; 28 bytes hashed).
4. `DB = 00×106 || 01` (107 bytes; there is no salt).
5. `maskedDB = DB XOR MGF1(H, 107)`.
6. Clear the most significant bit of `maskedDB[0]`.
7. `EM = maskedDB || H || BC` (107 + 20 + 1 = 128 bytes).
8. `Sig = I2OSP( OS2IP(EM)^d mod n, 128 )`.

A library function "RSA-PSS sign, SHA-1, MGF1-SHA-1, salt length 0" on a
1024-bit key gives the same bytes, so you can use one. The output is
deterministic.

---

## 6. Message 2: handshake response (device to host)

### 6.1 Outer layout

The total length is **968 bytes**.

| Offset | Size | Value | Meaning |
|--------|------|-------|---------|
| 0 | 2 | `02 02` | Message type marker (message 2). Must match. |
| 2 | 2 | `00 80` | BE16 length of the wrapped key = 128. Must be 128. (The reference checks only byte 3.) |
| 4 | 128 | `C_k` | RSA-OAEP-wrapped session key |
| 132 | 4 | `00 00 03 40` | BE32 payload length = 832. (The reference checks only bytes 134 and 135.) |
| 136 | 832 | `C_p` | AES-128-CBC encrypted payload |

### 6.2 Processing

1. **Unwrap the session key (RSAES-OAEP, SHA-1).**
   `EM = I2OSP( OS2IP(C_k)^d mod n, 128 )`. Keep any leading zero bytes, so
   that `EM` is exactly 128 bytes.
   - `maskedSeed = EM[1..21)`, `maskedDB = EM[21..128)` (107 bytes).
   - `seed = maskedSeed XOR MGF1(maskedDB, 20)`.
   - `DB = maskedDB XOR MGF1(seed, 107)`.
   - `K_s = DB[91..107)`, the **last 16 bytes of DB** (which are `EM[112..128)` after unmasking).

   Standard OAEP layout (with SHA-1 and a 16-byte message): `DB = lHash(20) ||
   00×70 || 01 || K_s`. The reference does **not** check `EM[0] == 00`,
   `lHash` or the `01` separator. It only takes the last 16 bytes. A synthetic
   test shows that a standard "RSA-OAEP decrypt, SHA-1, MGF1-SHA-1, empty
   label" call gives the same 16 bytes when the device used an empty label.
   The label the device uses was not confirmed. **Recommendation:** do the
   raw RSA operation and the unmasking yourself, as described above. Then
   check the separator structure only to log a warning, not to reject the
   message.

2. **Decrypt the payload.** `P = AES-128-CBC-Decrypt(key = K_s, IV = 00×16, C_p)`.
   This is 832 bytes (52 blocks). There is no padding scheme. Unused bytes at
   the end are zero.

3. **Parse `P`**. Use the length fields, not fixed offsets:

| Field | Size | Meaning / check |
|-------|------|-----------------|
| marker | 1 | `01` seen (not checked by the reference) |
| certLen | 4 | BE32 length of the device certificate chain (`0x026A` = 618 seen) |
| certs | certLen | Device certificate chain. The format is in Section 3.1 (count byte, then records; seen: "Zune Device CA 1", "Zune Device"). |
| hostRandLen | 2 | BE16, `00 10` |
| hostRand | hostRandLen | **Must be the same as `R_h`** (compare the 16 bytes). Reject if it is not. |
| devRandLen | 2 | BE16, `00 10` seen |
| devRand | devRandLen | Device random (not used afterward) |
| marker | 1 | `01` seen |
| sigLen | 2 | BE16, `00 80` seen |
| devSig | sigLen | Device signature (not checked by the reference) |
| marker | 1 | `01` seen |
| macLen | 2 | BE16, `00 20` (32) seen |
| `S` (MAC seed) | macLen | Used in Sections 7 and 8. Bytes `S[0..16)` and `S[16..20)` are used. |
| padding | rest | Zero bytes (7 seen) up to 832 |

   Check the sizes: 1+4+618+2+16+2+16+1+2+128+1+2+32 = 825, plus 7 bytes of
   padding = 832.

   Reject the message if any length field would read past 832, or if
   `macLen < 20`.

4. **Device authenticity (optional).** The reference does **not** check the
   device certificate chain or `devSig`. A public writeup says that `devSig`
   is made "in a way similar to" the host signature and can be checked with
   the public key in the "Zune Device" certificate. The exact signed region
   is not known. See Uncertainties. You can make the handshake work without
   this check.

The writeup calls `S` a "CBC-MAC / SHA-256 hash" over random data. The host
does not need to compute it again. It only uses `S` as key material.

---

## 7. Message 3: confirmation (host to device)

The total length is **20 bytes**.

| Offset | Size | Value |
|--------|------|-------|
| 0 | 2 | `02 03` (message type marker) |
| 2 | 2 | `00 10` (BE16 length = 16) |
| 4 | 16 | `CMAC( key = S[0..16), message = 00×15 || 01 )` |

The CMAC message is one full 16-byte block: fifteen zero bytes, then `01`.
Because it is a full block, CMAC uses subkey K1.

---

## 8. EnableTrustedFilesOperations parameters

1. `M = S[16..20)`. These are 4 bytes, used in the order they are in `S`
   (do not change their endianness).
2. `T = CMAC( key = S[0..16), message = M )`. The message is 4 bytes, which
   is not a full block, so CMAC pads it with `80 00…` and uses subkey K2.
3. `p_i = BE32( T[4i .. 4i+4) )` for i = 0..3.
4. Send operation `0x9214` with parameters `p_0, p_1, p_2, p_3`, in that
   order. (In the USB container, each parameter is written little-endian as
   usual in PTP. The *values* are the big-endian readings of the CMAC bytes.)

`K_s` is used only to decrypt `C_p`. After the handshake, the session key and
`S` are not used for anything more in the reference flow. The reference also
never calls DisableTrustedFilesOperations or EndTrustedAppSession at
teardown.

---

## 9. Crypto primitives: summary

| Primitive | Where | Details |
|-----------|-------|---------|
| RSA-1024 raw private op `x^d mod n` | Message 1 signature; Message 2 key unwrap | Big-endian I2OSP/OS2IP, output always 128 bytes (left-pad zeros) |
| EMSA-PSS (SHA-1, MGF1-SHA-1, sLen 0, emBits 1023) | Message 1 | Standard. Signed region is offsets 2..653. |
| OAEP decode (SHA-1, MGF1-SHA-1) | Message 2 key unwrap | Standard. Reference skips integrity checks and takes the last 16 bytes of DB. |
| SHA-1 | PSS, MGF1 | Standard FIPS 180 (checked against the "abc" test vector) |
| AES-128-CBC, IV = 0, no padding | Message 2 payload decrypt | Standard (checked against the FIPS-197 / SP 800-38A vectors) |
| AES-CMAC (RFC 4493) | Message 3; 0x9214 params | Standard. Only messages of 16 bytes and 4 bytes are used. |

There are **no nonstandard constructions**. The reference has its own SHA-1
and AES code, and its AES key schedule keeps a decryption schedule with
InvMixColumns applied (the "equivalent inverse cipher"). Both give the
standard FIPS results, so any conforming library can be used.

---

## 10. Test vectors / verification

All vectors below are **synthetic**. They were made from published standards
or from made-up inputs. None of them contain MTPZ key material. The
dirty-room author computed each one with an independent Python/OpenSSL
reference and checked it against the behavior of the reference
implementation.

### 10.1 Primitive checks (published)

- SHA-1("abc") = `a9993e364706816aba3e25717850c26c9cd0d89d`
- AES-CMAC, RFC 4493, key `2b7e151628aed2a6abf7158809cf4f3c`:
  - empty message: `bb1d6929e95937287fa37d129b756746` (exercises the K2/padding path)
  - message `6bc1bee22e409f96e93d7e117393172a`: `070a16b46b4d4144f79bdd9dd04a287c` (K1 path)
- MGF1-SHA-1(seed = ASCII "abc", 107) =
  `a03eb8ac3e6ad809805ec489ce85f2cefe8d00c69595e19565aec43db343f6e8c11e1224a7617f6f8d54d36ca26d2a23f0224a86d02eccd714a4cff5e64c0a21d72629cc0716386a9d168fccd8fedb83a2936c2ed7aaa104f502b65560480a7296d6011d8ded2246892bcb`

### 10.2 AES-128-CBC with zero IV

- key `000102030405060708090a0b0c0d0e0f`, IV `00…00` (16 bytes)
- plaintext = the bytes `00 01 02 … 1f` (32 bytes)
- ciphertext `0a940bb5416ef045f1c39458c653ea5a3cf456b4ca488aa383c79c98b34797cb`

### 10.3 PSS encoding for Message 1 (key-independent)

Synthetic inputs: a certificate chain of 629 bytes where byte `i` is
`i mod 256`, and `R_h = 00 01 02 … 0f`.

- `T` = `01 00 00 02 75` || chain || `00 10` || R_h (652 bytes)
- `mHash = SHA1(T)` = `979c9d15d6eb95f44444c1648b6d390e92225ccc`
- `H = SHA1(00×8 || mHash)` = `74987776b6d6a2590a1ceb9b1dc8b93064d89ef1`
- `EM` (128 bytes) =
  `6851f14e28c87bacc2493b9923f3d47b439bcdbfcd6e7040ef65bd4bfb0c3bd3827c1ae95b5c1ee232e7104d3f2ca5d27adc94d81778b7001c95d96c370ff2a69a76a6078df1971b86169a9395a988421a868791a31ebfaff88caae6be7a6c4f648c3575aaa55c417d685074987776b6d6a2590a1ceb9b1dc8b93064d89ef1bc`

Check: sign `T` with any throwaway RSA-1024 key using a library PSS
(SHA-1, salt 0). A raw public operation on the signature must give back
exactly this `EM`. (The dirty-room author did this check with OpenSSL.)

### 10.4 OAEP unwrap (key-independent: start from the EM after the RSA operation)

`EM` (128 bytes) =
`00701850a18db9d1f69bbb79c77603f6ddae3fcc019d099602de3ca53f8e8de34a2111526ba05b6e5c6397d1c1dbefc476bcaf21ab02161aa67558b1c530ff52c17e7f6e1ab40851c480acc98f56732b39a2085efdf18ddcd840f4ef8dd446990211d087593c2a6bc3dd15e8acc649a66bf7efdd37a6380841c6849309194ac8`

The unmasking in Section 6.2 must give:
- seed = `404142434445464748494a4b4c4d4e4f50515253`
- `DB[0..20)` = SHA1("") = `da39a3ee5e6b4b0d3255bfef95601890afd80709`, then 70 zero bytes, then `01`
- `K_s` = `101112131415161718191a1b1c1d1e1f`

### 10.5 Confirmation and EnableTrustedFilesOperations

Synthetic MAC seed `S` = bytes `a0 a1 a2 … bf` (32 bytes; `S[i] = 0xA0 + i`).

- Message 3 = `02 03 00 10` || `2ae71a77ea97545d740f70471bae9e43`
- CMAC(`S[0..16)`, `b0 b1 b2 b3`) = `792d4e1fafa730cc453d422aa50ed4f6`
- 0x9214 params: `p0 = 0x792d4e1f`, `p1 = 0xafa730cc`, `p2 = 0x453d422a`, `p3 = 0xa50ed4f6`

### 10.6 End-to-end without a device

You can test the full host side against a simulated device. Make a throwaway
RSA-1024 key pair. Put its n/d into a fake key file with a fake 629-byte
chain. In the simulated device:

1. Check the Message 1 signature with the public key (PSS, SHA-1, salt 0, over offsets 2..653).
2. Choose `K_s`, then OAEP-wrap it (SHA-1, empty label) to the public key.
3. Build a payload as described in 6.2. Zero-pad it to 832 bytes. Encrypt it with AES-128-CBC, IV 0.
4. Return Message 2.
5. Check that Message 3 and the 0x9214 params match a CMAC computed
   separately over the `S` you chose.

---

## 11. Uncertainties

1. **OAEP label / checks.** The reference does not check the OAEP structure
   (leading `00`, `lHash`, `01` separator). It is not confirmed that real
   devices use an empty label. Strict library OAEP decryption could reject
   valid device messages. Use the manual unmasking.
2. **Device signature and certificate validation.** The region `devSig`
   covers, its padding scheme, and how the device certificate chain is
   rooted are not known. No public source confirms them. The reference skips
   this check.
3. **Meaning and derivation of `S`.** It is described only as a "CBC-MAC /
   SHA-256 hash" (32 bytes). Its exact derivation is not known and not
   needed. Only `S[0..20)` is used. It is not known why `S[16..20)` is the
   CMAC message for 0x9214. It could be a counter. The reference uses these
   bytes unchanged.
4. **Session Initiator Info.** It is not known whether the device needs
   property `0xD406` to be set, or whether it checks the content. The
   reference sets it to its own identifying string before every handshake.
   You can send any reasonable string. If a device refuses the handshake, try
   different strings.
5. **Ignored header bytes.** The reference does not check Message 2 bytes 2
   and 132 to 133, or the 1-byte markers inside the payload. The values given
   here (`00`, `00 00`, `01`) are from one public capture.
6. **Ordering in the session.** The reference does the handshake after
   OpenSession and GetStorageIDs/GetStorageInfo. It is not known whether
   storage enumeration before the handshake is required, or whether earlier
   is also acceptable.
7. **Classic (flash/HDD) Zunes and Windows Phone 7.** The reference uses the
   same flow for all devices that show `microsoft.com/MTPZ` (the Zune
   `045e:0710` log shows that extension). No per-model differences are known.
   This was not checked on hardware.
8. **Which operations need the trusted state.** It is not known exactly which
   operations fail before 0x9214 succeeds (probably SendObjectInfo/
   SendObjectPropList/SendObject and deletes). It is also not known whether
   the trusted state ends after a timeout, at CloseSession, or at disconnect.
   Do the handshake again for each new session.
9. **Parameter count for 0x9215, and 0x9217 to 0x921D.** These are not used.
   Their semantics are not known.
10. **Key file line 2.** The cipher mode for the original-software blobs is
    not needed. It is described only from the writeup and from an unused
    routine in the reference (AES-128 per block, no chaining).

---

## 12. Sources

- libmtp (LGPL-2.1+), https://github.com/libmtp/libmtp, `master` branch, read November 2026:
  - `src/mtpz.c`: handshake, key-file loading, crypto routines
  - `src/mtpz.h`
  - `src/ptp.h`: operation codes `0x9212` to `0x9216`, property `0xD406`
  - `src/ptp.c`: data-phase direction of `0x9212`/`0x9213`
  - `src/libmtp.c`: where the handshake is called (extension detection, place in the device-open sequence) and how VendorExtensionDesc is parsed
  - `src/music-players.h`: Zune USB IDs
- Sajid Anwar's MTPZ notes, `mtpz.md` in https://github.com/kbhomes/libmtp-zune.
  These are prose notes on message layouts, certificate records, the
  key-blob origin and the response payload fields. The copy has real key
  material in its hex dumps. **None of that was copied.** The scratchpad copy
  was deleted after it was read.
- Device logs in the same repository: `logs/mtp-detect-microsoft-zune-hd.txt`
  and `logs/mtp-detect-zune.txt` (VendorExtensionDesc, OS descriptor,
  supported operations and properties).
- Standards: FIPS 180-4 (SHA-1), FIPS 197 (AES), NIST SP 800-38A (CBC),
  NIST SP 800-38B / RFC 4493 (CMAC), RFC 8017 / PKCS #1 v2.1 (RSASSA-PSS,
  RSAES-OAEP, MGF1), and the MTP 1.0 specification (containers,
  SetDevicePropValue, string datatype).

### Provenance

This specification was written by the "dirty room" in a clean-room process.
The author read the sources listed above and described the protocol in their
own words, as facts about the wire format and the cryptography. No source
code, comments, identifiers or function structure from libmtp were copied.
Only interoperability constants (opcodes, marker bytes, lengths) are given.
The test vectors were made from synthetic inputs and computed with an
independent reference written from public standards. Then they were compared
with the behavior of the reference implementation. This document contains no
MTPZ key material.
