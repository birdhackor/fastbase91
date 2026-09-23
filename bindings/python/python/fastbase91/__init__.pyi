from typing import Protocol


class ReadableBuffer(Protocol):
    def __buffer__(self, flags: int, /) -> memoryview: ...


class DecodeError(ValueError):
    """Raised when decoding fails.

    ``offset`` is the 0-based byte position of the failure (whole-stream for the
    streaming :class:`Decoder`); ``byte`` is the offending out-of-alphabet byte.
    """

    offset: int
    byte: int


def encode(data: ReadableBuffer, /) -> bytes: ...
def decode(data: ReadableBuffer, /, *, strict: bool = False) -> bytes: ...


class Encoder:
    def __init__(self) -> None: ...
    def update(self, data: ReadableBuffer, /) -> bytes: ...
    def finish(self) -> bytes: ...


class Decoder:
    def __init__(self, *, strict: bool = False) -> None: ...
    def update(self, data: ReadableBuffer, /) -> bytes: ...
    def finish(self) -> bytes: ...
