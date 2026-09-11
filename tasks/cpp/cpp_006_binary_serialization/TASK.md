# CPP-006: Binary serialization

Complete the C++20 packet serializer/deserializer declared in
`include/packet.hpp`, without changing its public API.  Do not add external
dependencies.  Build with CMake and run the visible CTest suite.

`serialize` returns `std::nullopt` when `packet.payload.size()` exceeds
`hase::kMaxPayloadSize`; otherwise it returns the exact wire representation.
`deserialize` must never read beyond its supplied span.  On any failure it
returns an empty `packet` and the relevant non-`none` `DecodeError`; on success
it returns a packet and `DecodeError::none`.

The wire format is exactly, in byte order:

```text
offset  size  field
0       4     ASCII magic: H A S E
4       1     version: exactly 1
5       1     flags: copied unchanged
6       2     reserved: both bytes must be zero
8       4     payload length: unsigned 32-bit big-endian
12      8     timestamp: unsigned 64-bit big-endian
20      N     payload bytes
20+N    4     checksum: unsigned 32-bit big-endian
```

There must be exactly one packet in the span: extra trailing bytes are an
`invalid_header` error. Decode classification has this precedence. Before a
complete magic is available, a byte sequence that matches the corresponding
prefix of `HASE` (including the empty span) is `truncated`; a mismatching byte
is `invalid_header`. After a complete valid magic, any missing fixed-header
byte is `truncated`. `payload length` above `kMaxPayloadSize` is an
`impossible_length` error, including when the input is also short. Once the
fixed header is valid and the declared length is possible, a missing payload or
checksum byte is `truncated`. Invalid magic, version, reserved bytes, or
trailing bytes are `invalid_header`. A complete packet whose checksum does not
match is `checksum_mismatch`.

The checksum is 32-bit FNV-1a over every byte before the checksum field:
start with `2166136261`, then for each byte XOR its unsigned value and multiply
by `16777619`, with ordinary unsigned 32-bit wraparound.  The checksum itself
is not included in that calculation.

This task deliberately has a narrow format contract.  It does not require
streaming, schema evolution, cryptographic integrity, compression, native
endianness support, or accepting alternate packet versions.
