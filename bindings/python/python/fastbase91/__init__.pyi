from typing import Protocol

__version__: str
CORE_VERSION: str


class ReadableBuffer(Protocol):
    def __buffer__(self, flags: int, /) -> memoryview: ...


class DecodeError(ValueError):
    """A ``ValueError`` subclass raised in strict mode for a byte outside the basE91 alphabet.

    ``offset`` is the byte's 0-based position; for :class:`Decoder`, it is measured from the start of the whole stream. ``byte`` is the byte's value.
    """

    offset: int
    byte: int


def encode(data: ReadableBuffer, /) -> bytes:
    """Encode ``data`` as basE91 and return ``bytes``.

    The caller must not mutate a writable input buffer concurrently during this call.
    """


def decode(data: ReadableBuffer, /, *, strict: bool = False) -> bytes:
    """Decode basE91 ``data`` and return ``bytes``.

    Bytes outside the basE91 alphabet are ignored by default; in ``strict=True`` mode, the first such byte raises :class:`DecodeError`.

    The caller must not mutate a writable input buffer concurrently during this call.
    """


class Encoder:
    """Incrementally encode input as basE91.

    Call ``update()`` for each input chunk, then call ``finish()`` once at the end of the message.
    """

    def __init__(self) -> None: ...

    def update(self, data: ReadableBuffer, /) -> bytes:
        """Encode the next input chunk and return the available output.

        The caller must not mutate a writable input buffer concurrently during this call.
        """

    def finish(self) -> bytes:
        """Return the message's final output as ``bytes`` and close this encoder.

        Later calls to ``update()`` or ``finish()`` raise ``ValueError``.
        """


class Decoder:
    """Incrementally decode basE91 input.

    Bytes outside the basE91 alphabet are ignored by default; in ``strict=True`` mode, the first such byte raises :class:`DecodeError`.
    """

    def __init__(self, *, strict: bool = False) -> None: ...

    def update(self, data: ReadableBuffer, /) -> bytes:
        """Decode the next input chunk and return the available output.

        When strict mode rejects a byte, the call produces no output and leaves the decoder state unchanged.

        The caller must not mutate a writable input buffer concurrently during this call.
        """

    def finish(self) -> bytes:
        """Return the final decoded byte, if any, as ``bytes`` and close this decoder.

        Later calls to ``update()`` or ``finish()`` raise ``ValueError``.
        """
