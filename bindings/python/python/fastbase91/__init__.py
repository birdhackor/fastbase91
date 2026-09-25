"""Standard basE91 binary-to-text encoding, backed by a Rust core.

The extension declares free-threading support (``gil_used = false``), so
one-shot :func:`encode`/:func:`decode` calls can run in parallel on
free-threaded (``t``) CPython builds while the GIL stays disabled
(``sys._is_gil_enabled()`` is ``False``). Sub-interpreters are **not** supported:
importing this package under a per-interpreter GIL (``concurrent.interpreters``)
raises ``ImportError``. A sub-interpreter that shares the main interpreter's GIL
imports without an error, but the extension's classes and exception type are the
same objects in every interpreter, not isolated copies. Use one
:class:`Encoder`/:class:`Decoder` per stream or thread; a concurrent ``update``
on the same instance can raise ``RuntimeError``.
"""

from ._fastbase91 import (
    CORE_VERSION,
    DecodeError,
    Decoder,
    Encoder,
    __version__,
    decode,
    encode,
)

__all__ = [
    "CORE_VERSION",
    "DecodeError",
    "Decoder",
    "Encoder",
    "__version__",
    "decode",
    "encode",
]
