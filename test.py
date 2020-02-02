import time
import rdflib
import reasonable

print(reasonable.__doc__)
print(reasonable.PyReasoner.__doc__)

# using rdflib
g = rdflib.Graph()
g.parse("example_models/ontologies/Brick.n3", format="n3")
g.parse("example_models/small1.n3", format="n3")

r1 = reasonable.PyReasoner()
r1.from_graph(g)
triples = r1.reason()
print("from rdflib:", len(triples))

# native
r = reasonable.PyReasoner()
print(r)
r.load_file("example_models/ontologies/Brick.n3")
r.load_file("example_models/small1.n3")
t0 = time.time()
triples = r.reason()
t1 = time.time()
print("from native run 1:", len(triples), t1-t0)

t0 = time.time()
triples = r.reason()
t1 = time.time()
print("from native run 2:", len(triples), t1-t0)
