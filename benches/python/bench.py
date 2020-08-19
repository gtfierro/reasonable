import rdflib
import glob
import brickschema
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


# data_files = [
#     "../../example_models/small1.n3",
#     "../../example_models/soda_hall.n3",
# ]
data_files = glob.glob("buildings/*.ttl")
data_sizes = {}
for f in data_files:
    g = rdflib.Graph()
    g.parse(f, format='ttl')
    data_sizes[f] = len(g)

N = 1

all_samples = {
    'owlrl': {},
    'allegro': {},
    'owlready2': {},
    'reasonable': {},
}


# owlready2
import owlready2
for data_file in data_files:
    print(f"Benching owlready2 on {data_file}")
    all_samples['owlready2'][data_file] = []
    for i in range(N):
        g = rdflib.Graph()
        load_file(g, data_file)
        for ont_file in ontology_files:
            load_file(g, ont_file)
        g.serialize("_owlready2_input.ttl", format="ttl")
        world = owlready2.World()
        onto = world.get_ontology(f"file://./_owlready2_input.ttl").load()
        t0 = time.time()
        owlready2.sync_reasoner(world, infer_property_values =True)
        G = world.as_rdflib_graph()
        t1 = time.time()
        print(f"    owlready2: {data_file} took {t1-t0}")
        all_samples['owlready2'][data_file].append({'duration': t1-t0, 'triples': len(G)})


# OWLRL
import owlrl
for data_file in data_files:
    print(f"Benching owlrl on {data_file}")
    all_samples['owlrl'][data_file] = []
    for i in range(N):
        g = rdflib.Graph()
        load_file(g, data_file)
        for ont_file in ontology_files:
            load_file(g, ont_file)

        t0 = time.time()
        owlrl.DeductiveClosure(owlrl.OWLRL_Semantics).expand(g)
        t1 = time.time()
        print(f"    owlrl: {data_file} took {t1-t0}")
        all_samples['owlrl'][data_file].append({'duration': t1-t0, 'triples': len(g)})

# Allegro
from brickschema.inference import OWLRLAllegroInferenceSession
for data_file in data_files:
    print(f"Benching owlrl on {data_file}")
    all_samples['allegro'][data_file] = []
    for i in range(N):
        g = rdflib.Graph()
        load_file(g, data_file)
        for ont_file in ontology_files:
            load_file(g, ont_file)
        sess = OWLRLAllegroInferenceSession()

        t0 = time.time()
        g = sess.expand(g)
        t1 = time.time()
        print(f"    allegro: {data_file} took {t1-t0}")
        all_samples['allegro'][data_file].append({'duration': t1-t0, 'triples': len(g)})

# reasonable
import reasonable
for data_file in data_files:
    print(f"Benching reasonable on {data_file}")
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
        print(f"    reasonable: {data_file} took {t1-t0}")
        all_samples['reasonable'][data_file].append({'duration': t1-t0, 'triples': len(triples)})


import pandas as pd
records = []
for reasoner, defn in all_samples.items():
    for data_file, samples in defn.items():
        for sample in samples:
            records.append((reasoner, data_file, sample['duration'], sample['triples']))
df = pd.DataFrame.from_records(records)
df.columns=['reasoner','data_file','duration','triples']
df.loc[:, 'src_size'] = df['data_file'].map(lambda x: data_sizes[x])
df.to_csv('benchmark.csv', header=True, index=False)
