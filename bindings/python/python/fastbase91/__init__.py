"""Python facade for the fastbase91 native extension."""

from ._fastbase91 import DecodeError, Decoder, Encoder, decode, encode

__all__ = ["DecodeError", "Decoder", "Encoder", "decode", "encode"]
