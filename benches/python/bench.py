import rdflib
import time

def load_file(g, f):
    if f.endswith('.ttl'):
        g.parse(f, format='ttl')
    elif f.endswith('.n3'):
        g.parse(f, format='ntriples')

ontology_files = [
    "../../example_models/ontologies/Brick.n3",
    "../../example_models/ontologies/owl.n3",
    "../../example_models/ontologies/rdfs.ttl",
]

data_files = [
    "../../example_models/small1.n3",
    "../../example_models/soda_hall.n3",
]

N = 100

all_samples = {
    'owlrl': {},
    'owlready2': {},
    'reasonable': {},
}

# OWLRL
import owlrl
for data_file in data_files:
    all_samples['owlrl'][data_file] = []
    for i in range(N):
        g = rdflib.Graph()
        load_file(g, data_file)
        for ont_file in ontology_files:
            load_file(g, ont_file)

        t0 = time.time()
        owlrl.DeductiveClosure(owlrl.OWLRL_Semantics).expand(g)
        t1 = time.time()
        print(f"owlrl: {data_file} took {t1-t0}")
        all_samples['owlrl'][data_file].append(t1-t0)


# reasonable
import reasonable
for data_file in data_files:
    all_samples['reasonable'][data_file] = []
    for i in range(N):
        g = rdflib.Graph()
        load_file(g, data_file)
        for ont_file in ontology_files:
            load_file(g, ont_file)
        r = reasonable.PyReasoner()
        r.from_graph(g)

        t0 = time.time()
        triples = r.reason()
        t1 = time.time()
        print(f"reasonable: {data_file} took {t1-t0}")
        all_samples['reasonable'][data_file].append(t1-t0)


import pandas as pd
records = []
for reasoner, defn in all_samples.items():
    for data_file, samples in defn.items():
        for sample in samples:
            records.append((reasoner, data_file, sample))
df = pd.DataFrame.from_records(records)
df.columns=['reasoner','data_file','duration']
