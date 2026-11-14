"""Module entrypoint for ``python -m satya_ml.texture_gen``."""

import sys

from satya_ml.texture_gen.cli import main

if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
