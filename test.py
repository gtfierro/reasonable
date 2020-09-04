import time
import rdflib
import reasonable

print(reasonable.__doc__)
print(reasonable.PyReasoner.__doc__)

# using rdflib
# g = rdflib.Graph()
# g.parse("example_models/ontologies/Brick.n3", format="n3")
# g.parse("example_models/small1.n3", format="n3")

# r1 = reasonable.PyReasoner()
# r1.from_graph(g)
# triples = r1.reason()
# for t in triples:
#     g.add(t)
# print("from rdflib:", len(triples))

# native
r = reasonable.PyReasoner()
print(r)
r.load_file("example_models/ontologies/Brick.n3")
r.load_file("brick_inference_test.n3")
t0 = time.time()
triples = r.reason()
t1 = time.time()
g = rdflib.Graph()
for t in triples:
    g.add(t)
print("from native run 1:", len(triples), t1-t0)
#
# t0 = time.time()
# triples = r.reason()
# t1 = time.time()
# print("from native run 2:", len(triples), t1-t0)

g.bind('brick', rdflib.Namespace("https://brickschema.org/schema/1.1/Brick#"))
sensors = g.query("SELECT ?s WHERE { ?s rdf:type brick:Temperature_Sensor }")
print(len(sensors))
print(list(sensors))

g.bind('brick', rdflib.Namespace("https://brickschema.org/schema/1.1/Brick#"))
sensors = g.query("SELECT ?s WHERE { ?s brick:measures brick:Temperature }")
print(len(sensors))
print(list(sensors))
