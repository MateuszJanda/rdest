#!/usr/bin/env python3

import hashlib

PIECE_SIZE = 2**18


def main():
    data = b"a" * (PIECE_SIZE + 1)

    piece1 = data[:PIECE_SIZE]
    piece2 = data[PIECE_SIZE:]

    with open("file-data.iso", "wb") as f:
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
