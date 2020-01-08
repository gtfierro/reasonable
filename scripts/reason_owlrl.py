#!/usr/bin/env python3

import owlrl
import rdflib
import sys

g = rdflib.Graph()

def load_file(f):
    if f.endswith('.ttl'):
        g.parse(f, format='ttl')
    elif f.endswith('.n3'):
        g.parse(f, format='ntriples')
for f in sys.argv[1:]:
    load_file(f)
owlrl.DeductiveClosure(owlrl.OWLRL_Semantics).expand(g)
