import random
import sys
from concurrent.futures import ThreadPoolExecutor

import pytest

import fastbase91
from fastbase91 import DecodeError, Decoder, Encoder, decode, encode


def _random_bytes(seed, length):
    generator = random.Random(seed)
    return bytes(generator.randrange(256) for _ in range(length))


ROUND_TRIP_CASES = [
    b"",
    b"plain ASCII text",
    bytes([0x00, 0xFF, 0x00, 0xFF]),
    _random_bytes(91, 257),
    _random_bytes(9_191, 128 * 1024),
]


@pytest.mark.parametrize(
    "data",
    ROUND_TRIP_CASES,
    ids=["empty", "ascii", "edge-bytes", "random-257", "large-random"],
)
def test_round_trip(data):
    assert decode(encode(data)) == data


def test_named_hello_vector():
    assert encode(b"hello") == b"TPwJh>A"
    assert decode(b"TPwJh>A") == b"hello"


def test_lenient_whitespace_and_strict_structured_error():
    assert decode(b"TPw Jh>A") == b"hello"

    with pytest.raises(DecodeError) as caught:
        decode(b"TPw Jh>A", strict=True)

    error = caught.value
    assert type(error.offset) is int
    assert type(error.byte) is int
    assert error.offset == 3
    assert error.byte == 0x20
    assert "invalid byte 0x20 at offset 3" in str(error)
    assert isinstance(error, ValueError)


def test_decode_error_is_a_value_error_subclass():
    assert issubclass(DecodeError, ValueError)
    assert "DecodeError" in fastbase91.__all__


@pytest.mark.parametrize("buffer_type", [bytes, bytearray, memoryview])
def test_bytes_like_buffers(buffer_type):
    plain = buffer_type(b"hello")
    encoded = buffer_type(b"TPwJh>A")
    encoded_result = encode(plain)
    decoded_result = decode(encoded)
    assert type(encoded_result) is bytes
    assert type(decoded_result) is bytes
    assert encoded_result == b"TPwJh>A"
    assert decoded_result == b"hello"


def _chunks(data, widths):
    start = 0
    for width in widths:
        if start >= len(data):
            break
        end = min(start + width, len(data))
        yield data[start:end]
        start = end
    if start < len(data):
        yield data[start:]


def test_streaming_matches_one_shot_across_chunk_boundaries():
    plain = _random_bytes(191, 16_401)

    encoder = Encoder()
    encoded_parts = [
        encoder.update(chunk)
        for chunk in _chunks(plain, [1, 2, 13, 3, 257, 4_096, 7])
    ]
    encoded_parts.append(encoder.finish())
    encoded = b"".join(encoded_parts)
    assert encoded == encode(plain)

    decoder = Decoder()
    decoded_parts = [
        decoder.update(chunk)
        for chunk in _chunks(encoded, [1, 3, 2, 17, 1_024, 5, 4_097])
    ]
    decoded_parts.append(decoder.finish())
    assert b"".join(decoded_parts) == plain
    assert b"".join(decoded_parts) == decode(encoded)


def test_streaming_accepts_bytes_like_buffers():
    encoder = Encoder()
    encoded = encoder.update(bytearray(b"hel"))
    encoded += encoder.update(memoryview(b"lo"))
    encoded += encoder.finish()

    decoder = Decoder()
    decoded = decoder.update(memoryview(encoded[:3]))
    decoded += decoder.update(bytearray(encoded[3:]))
    decoded += decoder.finish()
    assert decoded == b"hello"


def test_streaming_decode_error_offset_is_chunk_local():
    decoder = Decoder(strict=True)
    decoder.update(b"TPw")

    with pytest.raises(DecodeError) as caught:
        decoder.update(b"J >A")

    assert caught.value.offset == 1
    assert caught.value.byte == 0x20


@pytest.mark.parametrize("codec", [Encoder, Decoder])
def test_finish_closes_stream(codec):
    stream = codec()
    stream.finish()

    with pytest.raises(ValueError, match="closed"):
        stream.update(b"data")
    with pytest.raises(ValueError, match="closed"):
        stream.finish()


def _free_threaded_runtime():
    is_gil_enabled = getattr(sys, "_is_gil_enabled", None)
    return is_gil_enabled is not None and not is_gil_enabled()


@pytest.mark.skipif(
    not _free_threaded_runtime(),
    reason="requires a free-threaded Python runtime with the GIL disabled",
)
def test_free_threaded_runtime_round_trip():
    data = ROUND_TRIP_CASES[-1]
    with ThreadPoolExecutor(max_workers=4) as executor:
        results = list(executor.map(lambda _: decode(encode(data)), range(8)))
    assert results == [data] * 8
