#!/bin/sh
# Select Z3's integer encoding of bit-vector obligations.
exec z3 sat.smt=true smt.bv.solver=2 "$@"
