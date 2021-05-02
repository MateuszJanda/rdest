#!/usr/bin/env python3

# Copyright 2020 Mateusz Janda.
#
# Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
# https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
# <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
# option. This file may not be copied, modified, or distributed
# except according to those terms.

import hashlib

PIECE_SIZE = 2**18


def main():
    data = b"a" * (PIECE_SIZE + 1)

    piece1 = data[:PIECE_SIZE]
    piece2 = data[PIECE_SIZE:]

    with open("file-data1.iso", "wb") as f:
        f.write(data)

    save_piece(piece1)
    save_piece(piece2)


def save_piece(piece):
    h = sha1_hash(piece)
    with open(h + ".piece", "wb") as f:
        f.write(piece)


def sha1_hash(piece):
    m = hashlib.sha1()
    m.update(piece)
    return m.digest().hex().upper()


if __name__ == '__main__':
    main()
