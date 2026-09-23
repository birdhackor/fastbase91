"""Python facade for the fastbase91 native extension scaffold."""

from ._fastbase91 import Decoder, Encoder, decode, encode

__all__ = ["Decoder", "Encoder", "decode", "encode"]
