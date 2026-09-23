import random
import sys
import sysconfig
from array import array
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


def test_data_parameters_are_positional_only():
    keyword_calls = [
        lambda: encode(data=b"x"),
        lambda: decode(data=b"x"),
        lambda: Encoder().update(data=b"A"),
        lambda: Decoder().update(data=b"A"),
    ]
    for call in keyword_calls:
        with pytest.raises(TypeError):
            call()

    assert decode(encode(b"x")) == b"x"
    assert type(Encoder().update(b"A")) is bytes
    assert type(Decoder().update(b"A")) is bytes


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


def test_nonempty_bytes_input_round_trip():
    plain = b"immutable bytes input"
    assert decode(encode(plain)) == plain


def test_signed_byte_buffers_preserve_negative_value_bits_across_all_entry_points():
    signed_plain = array("b", [-128, -1, 0, 1, 65, 127])
    unsigned_plain = b"\x80\xff\x00\x01A\x7f"
    assert signed_plain.tobytes() == unsigned_plain

    one_shot_encoded = encode(memoryview(signed_plain))
    signed_encoded = array("b", one_shot_encoded)
    assert decode(memoryview(signed_encoded)) == unsigned_plain

    encoder = Encoder()
    streaming_encoded = encoder.update(memoryview(signed_plain)) + encoder.finish()
    assert streaming_encoded == one_shot_encoded

    decoder = Decoder()
    streaming_decoded = decoder.update(memoryview(signed_encoded)) + decoder.finish()
    assert streaming_decoded == unsigned_plain


def _encoder_update(data):
    return Encoder().update(data)


def _decoder_update(data):
    return Decoder().update(data)


@pytest.mark.parametrize(
    "consume",
    [encode, decode, _encoder_update, _decoder_update],
    ids=["encode", "decode", "encoder-update", "decoder-update"],
)
@pytest.mark.parametrize(
    "buffer_factory",
    [
        lambda: memoryview(b"abcdef")[::2],
        lambda: memoryview(b"abcdef").cast("B", shape=(2, 3)),
    ],
    ids=["non-contiguous", "multi-dimensional"],
)
def test_rejects_unsupported_buffer_layouts(consume, buffer_factory):
    with pytest.raises(
        BufferError,
        match="v1 only accepts contiguous one-dimensional bytes-like objects",
    ):
        consume(buffer_factory())


@pytest.mark.parametrize(
    "consume",
    [encode, decode, _encoder_update, _decoder_update],
    ids=["encode", "decode", "encoder-update", "decoder-update"],
)
def test_rejects_non_byte_buffers(consume):
    with pytest.raises(
        BufferError,
        match="v1 only accepts signed or unsigned single-byte buffers",
    ):
        consume(memoryview(array("i", [1, 2, 3])))


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
    assert all(type(part) is bytes for part in encoded_parts)
    encoded = b"".join(encoded_parts)
    assert encoded == encode(plain)

    decoder = Decoder()
    decoded_parts = [
        decoder.update(chunk)
        for chunk in _chunks(encoded, [1, 3, 2, 17, 1_024, 5, 4_097])
    ]
    decoded_parts.append(decoder.finish())
    assert all(type(part) is bytes for part in decoded_parts)
    assert b"".join(decoded_parts) == plain
    assert b"".join(decoded_parts) == decode(encoded)


def test_streaming_returns_immediate_exact_bytes_prefixes():
    encoder = Encoder()
    encoded_parts = [
        encoder.update(b"he"),
        encoder.update(b"llo"),
        encoder.finish(),
    ]
    assert [type(part) for part in encoded_parts] == [bytes, bytes, bytes]
    assert encoded_parts == [b"TP", b"wJh>", b"A"]

    decoder = Decoder()
    decoded_parts = [
        decoder.update(b"TP"),
        decoder.update(b"wJh>A"),
        decoder.finish(),
    ]
    assert [type(part) for part in decoded_parts] == [bytes, bytes, bytes]
    assert decoded_parts == [b"h", b"ell", b"o"]


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


def test_streaming_decode_error_offset_is_whole_stream_and_state_is_transactional():
    decoder = Decoder(strict=True)
    decoded = decoder.update(b"TPw")

    with pytest.raises(DecodeError) as caught:
        decoder.update(b"J h>A")

    assert caught.value.offset == 4
    assert caught.value.byte == 0x20
    assert "invalid byte 0x20 at offset 4" in str(caught.value)

    with pytest.raises(DecodeError) as repeated:
        decoder.update(b"J h>A")
    assert repeated.value.offset == 4

    decoded += decoder.update(b"Jh>A")
    decoded += decoder.finish()
    assert decoded == b"hello"


@pytest.mark.parametrize("codec", [Encoder, Decoder])
def test_finish_closes_stream(codec):
    stream = codec()
    stream.finish()

    with pytest.raises(ValueError, match="closed"):
        stream.update(b"data")
    with pytest.raises(ValueError, match="closed"):
        stream.finish()


def _free_threaded_build():
    return sysconfig.get_config_var("Py_GIL_DISABLED") == 1


@pytest.mark.skipif(
    not _free_threaded_build(),
    reason="requires a free-threaded Python build (Py_GIL_DISABLED is false)",
)
def test_free_threaded_runtime_round_trip():
    assert not sys._is_gil_enabled(), "importing fastbase91 must not re-enable the GIL"
    data = ROUND_TRIP_CASES[-1]
    with ThreadPoolExecutor(max_workers=4) as executor:
        results = list(executor.map(lambda _: decode(encode(data)), range(8)))
    assert results == [data] * 8
