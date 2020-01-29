#!/usr/bin/env python3
from rdflib import Graph
import sys

g = Graph()
filename, ending = sys.argv[1].split('.')
if ending == 'ttl':
    g.parse(sys.argv[1], format='ttl')
elif ending == 'rdf':
    g.parse(sys.argv[1], format='xml')
with open(f"{filename}.n3", "wb") as f:
    f.write(g.serialize(format='ntriples'))
