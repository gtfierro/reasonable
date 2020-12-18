import reasonable
import rdflib

g = rdflib.Graph()
g.parse('Brick.ttl', format='ttl')
g.parse('example_models/small1.n3', format='ntriples')

r = reasonable.PyReasoner()
r.from_graph(g)
triples = r.reason()
print("from rdflib:", len(triples))
g2 = rdflib.Graph()
for t in triples:
    print(t)
    g2.add(t)
g2.serialize('output.ttl', format='ttl')
