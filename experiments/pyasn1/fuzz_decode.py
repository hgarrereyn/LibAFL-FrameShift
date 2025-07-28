import os
import sys
import atheris
import io
from pyasn1.codec.ber import decoder
from pyasn1 import error


def TestOneInput(data):
    stream = io.BytesIO(data)
    try:
        c = 0
        for value in decoder.StreamingDecoder(stream):
            s = str(value)
            c += 1
            # avoid infinite loops here
            if c > 100:
                break
    except error.PyAsn1Error:
        pass


def main():
    atheris.instrument_all()
    atheris.Setup(sys.argv, TestOneInput, enable_python_coverage=True)
    atheris.Fuzz()


if __name__ == "__main__":
    main()
