# Henke basE91 reference oracle

The only compatibility oracle in these tests is Joachim Henke's basE91 C
reference, version 0.6.0 (released 2006-11-02).

- Project: <https://base91.sourceforge.net/>
- Source archive: <https://sourceforge.net/projects/base91/files/basE91/0.6.0/base91-0.6.0.tar.gz/download>
- Archive SHA-256: `02cfae7322c1f865ca6ce8f2e0bb8d38c8513e76aed67bf1c94eab1343c6c651`
- License: 3-clause BSD; the unmodified upstream text is in
  `vendor/base91-0.6.0/LICENSE`.

`base91.c`, `base91.h`, and `LICENSE` are byte-for-byte copies from that
archive. Their hashes are recorded in `SHA256SUMS`.

`oracle.c` is a project-owned streaming command-line adapter. It accepts
`encode` or `decode` as its sole argument, reads arbitrary bytes from stdin,
and writes the reference result to stdout. The Rust helper in `mod.rs` compiles
the adapter and vendored source with the host C compiler, then invokes it via
`std::process::Command`. The same `decode` mode is ready for the decoder phase.

To verify the vendored files on macOS:

```console
$ shasum -a 256 vendor/base91-0.6.0/base91.c vendor/base91-0.6.0/base91.h vendor/base91-0.6.0/LICENSE
ce6c825e24342271b525f2cecc5f70294047246963cc5361003326c9d6ce8290  vendor/base91-0.6.0/base91.c
30d476141130aa6391a240c9b94e42e60f71b38f056a2a349fec9bc96562b6bf  vendor/base91-0.6.0/base91.h
51f70a0b8d3df8662f97469234af3c8fbf780b8d853568ff9d32b56d6d4b996b  vendor/base91-0.6.0/LICENSE
```
