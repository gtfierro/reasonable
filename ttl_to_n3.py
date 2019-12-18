from rdflib import Graph
import sys

g = Graph()
filename, ending = sys.argv[1].split('.')
g.parse(sys.argv[1], format='ttl')
with open(f"{filename}.n3", "wb") as f:
    f.write(g.serialize(format='ntriples'))
