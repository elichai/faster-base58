#!/bin/sh
# Normalize constant-multiply identities before bit-vector solving.
exec z3 rewriter.som=true rewriter.mul2concat=true "$@"
