#include <stdio.h>
#include <stdint.h>
#include "PerfectMatching.h"

extern "C" {
    void minimum_weight_perfect_matching(int node_num, int edge_num, int* edges, PerfectMatching::REAL* weights, int* match);
}

void minimum_weight_perfect_matching(int node_num, int edge_num, int* edges, PerfectMatching::REAL* weights, int* match) {
    PerfectMatching *pm = new PerfectMatching(node_num, edge_num);
    for (int e=0; e<edge_num; e++) {
        pm->AddEdge(edges[2*e], edges[2*e+1], weights[e]);
    }
    struct PerfectMatching::Options options;  // use default option
    options.verbose = false;
    pm->options = options;
    pm->Solve();
    for (int i=0; i<node_num; i++) {
		match[i] = pm->GetMatch(i);
	}
    delete pm;
}
