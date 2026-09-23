"""Standard basE91 binary-to-text encoding, backed by a Rust core.

The extension declares free-threading support (``gil_used = false``), so
one-shot :func:`encode`/:func:`decode` calls can run in parallel on
free-threaded (``t``) CPython builds. Sub-interpreters are **not** supported:
importing this package under a per-interpreter GIL (``concurrent.interpreters``)
raises ``ImportError``. Use one :class:`Encoder`/:class:`Decoder` per stream or
thread; a concurrent ``update`` on the same instance raises ``RuntimeError``.
"""

from ._fastbase91 import DecodeError, Decoder, Encoder, decode, encode

__all__ = ["DecodeError", "Decoder", "Encoder", "decode", "encode"]
