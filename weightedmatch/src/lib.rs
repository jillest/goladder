/* ************************************************************************* *
 *                                                                           *
 *        Copyright (c) 2004 Peter Cappello  <cappello@cs.ucsb.edu>          *
 *        Copyright (c) 2019 Jilles Tjoelker <jilles@stack.nl>               *
 *                                                                           *
 *    Permission is hereby granted, free of charge, to any person obtaining  *
 *  a copy of this software and associated documentation files (the          *
 *  "Software"), to deal in the Software without restriction, including      *
 *  without limitation the rights to use, copy, modify, merge, publish,      *
 *  distribute, sublicense, and/or sell copies of the Software, and to       *
 *  permit persons to whom the Software is furnished to do so, subject to    *
 *  the following conditions:                                                *
 *                                                                           *
 *    The above copyright notice and this permission notice shall be         *
 *  included in all copies or substantial portions of the Software.          *
 *                                                                           *
 *    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,        *
 *  EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF       *
 *  MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.   *
 *  IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY     *
 *  CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,     *
 *  TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE        *
 *  SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.                   *
 *                                                                           *
 * ************************************************************************* */

/*
 *  This class implements a [minimum | maximum] cost maximum matching based on
 *  an O(n^3) implementation of Edmonds' algorithm, as presented by Harold N.
 *  Gabow in his Ph.D. dissertation, Computer Science, Stanford University,
 *  1973.
 *
 * Created on July 8, 2003, 11:00 AM
 *
 * @author  Peter Cappello
 */
/*
 * Ported from Java to Rust in February 2019 by Jilles Tjoelker.
 */
/*
 * Gabow's implementation of Edmonds' algorithm is described in chapter 6 of
 * Nonbipartite Matching, of Combinatorial Optimization, Networks and Matroids,
 * authored by Eugene Lawler,
 * published by Holt, Rinehart, and Winston, 1976.
 * <p>
 * Lawler's description is referred to in the Notes and References section of
 * chapter 11, Weighted Matching, of
 * Combinatorial Optimation, Algorithms and Complexity,
 * authored by Christos Papadimitriou and Kenneth Steiglitz,
 * published by Prentice-Hall, 1982.
 * <p>
 * The implementation here mimics Gabow's description and Rothberg's C coding of
 * Gabow's description, making it easy for others to see the correspondence
 * between this code, Rothberg's C code, and Gabow's English description of the
 * algorithm, given in Appendix D of his dissertation.
 * <p>
 * Since the code mimics Gabow's description (Rothberg's C code does so even
 * more closely), the code below is not object-oriented, much less good Java. It
 * also violates many Java naming conventions.
 * <p>
 * Currently, the graph is assumed to be complete & symmetric.
 * <p>
 * It is unclear to me why cost values are doubled in set_up() and intialize(). I
 * think it may have to do with the implementation being entirely integer. When
 * I remove the doubling, the minimum weight maximum match fails on the test
 * graph.
 */

use std::mem;

// constants
/** The value that indicates that a minimum cost maximum match is sought. */
pub const MINIMIZE: bool = true;
/** The value that indicates that a maximum cost maximum match is sought. */
pub const MAXIMIZE: bool = false;

const DEBUG: bool = false;

type Edge = usize;
type Vertex = usize;
type Link = isize;
type Weight = i32;

const UNMATCHED: Edge = 0;

#[derive(Default)]
struct WeightedMatch {
    costs: Vec<Vec<Weight>>,

    max_v: usize,
    max_e: usize,
    dummy_vertex: Vertex,
    dummy_edge: Edge,

    a: Vec<usize>, // adjacency list
    end: Vec<usize>,
    mate: Vec<Edge>,
    weight: Vec<Weight>,

    base: Vec<usize>,
    last_edge: [Link; 3], // Used by methods that undo blossoms.
    last_vertex: Vec<usize>,
    link: Vec<Link>,
    next_delta: Vec<Weight>,
    next_edge: Vec<Edge>,
    next_pair: Vec<Edge>,
    next_vertex: Vec<Edge>,
    y: Vec<Weight>,

    delta: Weight,
    last_delta: Weight,
    new_base: usize,
    next_base: usize,
    stop_scan: usize,
    pair_point: usize,
    neighbor: usize,
    new_last: usize,
    next_point: Vertex,
    old_first: usize,
    second_mate: usize,
    f: Link,
    nxt_edge: Link,
    next_e: Link,
    next_u: usize,

    e: Link,
    v: Vertex,
    i: usize, // edge, vertex, index used by several methods.
}

impl WeightedMatch {
    /** Construct a WeightedMatch object. */
    fn new(costs: Vec<Vec<Weight>>) -> Self {
        Self {
            costs,
            ..Default::default()
        }
    }

    /** The int cost matrix is assumed to be square and symmetric (undirected).
     * <p>
     * if ( minimize_weight ) <br>
     * &nbsp;&nbsp;&nbsp;&nbsp; performs a minimum cost maximum matching;<br>
     * else<br>
     *    performs a maximum cost maximum matching.
     * @param minimize_weight if ( minimize_weight )
     *    performs a minimum cost maximum matching;
     * else
     *    performs a maximum cost maximum matching.
     * @return an array of the form vertex[i] = j, where vertex i is matched to vertex j.
     * The numbering of vertices is 1, ..., n, where the graph has n vertices. Thus,
     * the 0th element of the returned int[] is undefined.
     * <p>
     * I don't particularly like this, I am just propagating custom. I may change
     * this, at some point, so that vertices are numbered 0, ..., n-1.
     */
    fn weighted_match(&mut self, minimize_weight: bool) -> Vec<Edge> {
        if DEBUG {
            println!("weighted_match: input costs matrix:");
            for (i, row) in self.costs.iter().enumerate() {
                print!(" Row {}:", i);
                for val in row.iter().skip(i + 1) {
                    print!(" {}", val);
                }
                println!("");
            }
        }

        let mut loop_num = 1;

        // W0. Input.
        self.input();

        // W1. Initialize.
        self.initialize(minimize_weight);

        loop {
            if DEBUG {
                println!("\n *** A U G M E N T {}", loop_num);
                loop_num += 1;
            }
            // W2. Start a new search.
            self.delta = 0;
            for v1 in 1..=self.max_v {
                self.v = v1;
                if self.mate[self.v] == self.dummy_edge {
                    // Link all exposed vertices.
                    self.pointer(self.dummy_vertex, self.v, self.dummy_edge as Link);
                }
            }
            self.v = self.max_v + 1;

            if DEBUG {
                for q in 1..=self.max_v + 1 {
                    println!(
                        concat!(
                            "W2: i: {}",
                            ", mate: {}",
                            ", next_edge: {}",
                            ", next_vertex: {}",
                            ", link: {}",
                            ", base: {}",
                            ", last_vertex: {}",
                            ", y: {}",
                            ", next_delta: {}",
                            ", last_delta: {}"
                        ),
                        q,
                        self.mate[q],
                        self.next_edge[q],
                        self.next_vertex[q],
                        self.link[q],
                        self.base[q],
                        self.last_vertex[q],
                        self.y[q],
                        self.next_delta[q],
                        self.last_delta
                    );
                }
            }

            // W3. Get next edge.
            loop {
                self.i = 1;
                for j in 2..=self.max_v {
                    /* !!! Dissertation, p. 213, it is next_delta[i] < next_delta[j]
                     * When I make it <, the routine seems to do nothing.
                     */
                    if self.next_delta[self.i] > self.next_delta[j] {
                        self.i = j;
                    }
                }

                // delta is the minimum slack in the next edge.
                self.delta = self.next_delta[self.i];

                if DEBUG {
                    println!("\nW3: i: {} delta: {}", self.i, self.delta);
                }

                if self.delta == self.last_delta {
                    if DEBUG {
                        println!(
                            "\nW8: delta: {} last_delta: {}",
                            self.delta, self.last_delta
                        );
                    }
                    // W8. Undo blossoms.
                    self.set_bounds();
                    self.unpair_all();
                    for i in 1..=self.max_v {
                        self.mate[i] = self.end[self.mate[i]];
                        if self.mate[i] == self.dummy_vertex {
                            self.mate[i] = UNMATCHED;
                        }
                    }
                    self.mate.truncate(self.max_v + 1);

                    // W9.
                    return mem::replace(&mut self.mate, Vec::new());
                }

                // W4. Assign pair links.
                self.v = self.base[self.i];

                if DEBUG {
                    println!(
                        "W4. delta: {} v: {} link[v]: {}",
                        self.delta, self.v, self.link[self.v]
                    );
                }

                if self.link[self.v] >= 0 {
                    if self.pair() {
                        break;
                    }
                } else {
                    // W5. Assign pointer link.
                    if DEBUG {
                        println!("W5. delta: {} v: {}", self.delta, self.v);
                    }
                    let w = self.bmate(self.v); // blossom w is matched with blossom v.
                    if self.link[w] < 0 {
                        if DEBUG {
                            println!(
                                "WeightedMatch: delta: {} v: {} w: {} link[w]: {}",
                                self.delta, self.v, w, self.link[w]
                            );
                        }
                        // w is unlinked.
                        self.pointer(self.v, w, self.opp_edge(self.next_edge[self.i] as Link));
                    } else {
                        // W6. Undo a pair link.
                        if DEBUG {
                            println!("W6. v: {} w: {}", self.v, w);
                        }
                        self.unpair(self.v, w);
                    }
                }
            }

            // W7. Enlarge the matching.
            self.last_delta -= self.delta;
            self.set_bounds();
            let g = self.opp_edge(self.e);
            self.rematch(self.bend(self.e as usize), g);
            self.rematch(self.bend(g as usize), self.e);
        }
    }

    // Begin 5 simple functions
    //
    fn bend(&self, e: Edge) -> Edge {
        self.base[self.end[e]]
    }

    fn blink(&self, v: Vertex) -> Edge {
        self.base[self.end[self.link[v] as usize]]
    }

    fn bmate(&self, v: Vertex) -> Edge {
        self.base[self.end[self.mate[v]]]
    }

    fn opp_edge(&self, e: isize) -> isize {
        if (e - self.max_v as isize) % 2 == 0 {
            e - 1
        } else {
            e + 1
        }
    }

    fn slack(&self, e: Edge) -> Weight {
        if DEBUG {
            println!(
                "slack: e = {} y[end[e]] = {} weight[e] = {}",
                e, self.y[self.end[e]], self.weight[e]
            );
        }
        return self.y[self.end[e]] + self.y[self.end[self.opp_edge(e as Link) as usize]]
            - self.weight[e];
    }
    //
    // End 5 simple functions

    fn initialize(&mut self, minimize_weight: bool) {
        // initialize basic data structures
        self.set_up();

        if DEBUG {
            for q in 0..self.max_v + 2 * self.max_e + 2 {
                println!(
                    "initialize: i: {}, a: {} end: {} weight: {}",
                    q, self.a[q], self.end[q], self.weight[q]
                );
            }
        }

        self.dummy_vertex = self.max_v + 1;
        self.dummy_edge = self.max_v + 2 * self.max_e + 1;
        self.end[self.dummy_edge] = self.dummy_vertex;

        if DEBUG {
            println!(
                "initialize: dummy_vertex: {} dummy_edge: {} opp_edge(dummy_edge): {}",
                self.dummy_vertex,
                self.dummy_edge,
                self.opp_edge(self.dummy_edge as Link)
            );
        }

        let mut max_weight = Weight::min_value();
        let mut min_weight = Weight::max_value();
        for i in 0..self.max_v {
            for j in i + 1..self.max_v {
                let cost = 2 * self.costs[i][j];
                if cost > max_weight {
                    max_weight = cost;
                }
                if cost < min_weight {
                    min_weight = cost;
                }
            }
        }

        if DEBUG {
            println!(
                "initialize: min_weight: {}, max_weight: {}",
                min_weight, max_weight
            );
        }

        // If minimize costs, invert weights
        if minimize_weight {
            if self.max_v % 2 != 0 {
                panic!("|V| must be even for a minimum cost maximum matching.");
            }
            max_weight += 2; // Don't want all 0 weight
            for i in self.max_v + 1..=self.max_v + 2 * self.max_e {
                self.weight[i] = max_weight - self.weight[i];
                //println!("initialize: inverted weight[" + i + "]: " +
                //weight[i]);
            }
            max_weight = max_weight - min_weight;
        }

        self.last_delta = max_weight / 2;
        if DEBUG {
            println!(
                "initialize: min_weight: {} max_weight: {} last_delta: {}",
                min_weight, max_weight, self.last_delta
            );
        }

        let allocation_size = self.max_v + 2;
        self.mate = vec![0; allocation_size];
        self.link = vec![0; allocation_size];
        self.base = vec![0; allocation_size];
        self.next_vertex = vec![0; allocation_size];
        self.last_vertex = vec![0; allocation_size];
        self.y = vec![0; allocation_size];
        self.next_delta = vec![0; allocation_size];
        self.next_edge = vec![0; allocation_size];

        let allocation_size = self.max_v + 2 * self.max_e + 2;
        self.next_pair = vec![0; allocation_size];

        for i in 1..=self.max_v + 1 {
            self.mate[i] = self.dummy_edge;
            self.next_edge[i] = self.dummy_edge;
            self.next_vertex[i] = 0;
            self.link[i] = -(self.dummy_edge as Link);
            self.base[i] = i;
            self.last_vertex[i] = i;
            self.y[i] = self.last_delta;
            self.next_delta[i] = self.last_delta;

            if DEBUG {
                println!(
                    concat!(
                        "initialize: v: {}, i: {}",
                        ", mate: {}",
                        ", next_edge: {}",
                        ", next_vertex: {}",
                        ", link: {}",
                        ", base: {}",
                        ", last_vertex: {}",
                        ", y: {}",
                        ", next_delta: {}",
                        ", last_delta: {}"
                    ),
                    self.v,
                    i,
                    self.mate[i],
                    self.next_edge[i],
                    self.next_vertex[i],
                    self.link[i],
                    self.base[i],
                    self.last_vertex[i],
                    self.y[i],
                    self.next_delta[i],
                    self.last_delta
                );
            }
        }
        self.i = self.max_v + 2;
        //println!("initialize: complete.");
    }

    fn input(&mut self) {
        self.max_v = self.costs.len();
        self.max_e = self.max_v * (self.max_v - 1) / 2;

        let allocation_size = self.max_v + 2 * self.max_e + 2;
        self.a = vec![0; allocation_size];
        self.end = vec![0; allocation_size];
        self.weight = vec![0; allocation_size];

        if DEBUG {
            println!(
                "input: max_v: {}, max_e: {}, allocation_size: {}",
                self.max_v, self.max_e, allocation_size
            );
        }
    }

    /** Updates a blossom's pair list, possibly inserting a new edge.
     * It is invoked by scan and merge_pairs.
     * It is invoked with global int e set to the edge to be inserted, neighbor
     * set to the end vertex of e, and pair_point pointing to the next pair to be
     * examined in the pair list.
     */
    fn insert_pair(&mut self) {
        if DEBUG {
            println!(
                "Insert Pair e: {} {}- {}",
                self.e,
                self.end[self.opp_edge(self.e) as usize],
                self.end[self.e as usize]
            );
        }

        // IP1. Prepare to insert.
        let delta_e = self.slack(self.e as Edge) / 2;

        if DEBUG {
            println!("IP1: delta_e: {}", delta_e);
        }

        self.next_point = self.next_pair[self.pair_point];

        // IP2. Fint insertion point.
        while self.end[self.next_point] < self.neighbor {
            self.pair_point = self.next_point;
            self.next_point = self.next_pair[self.next_point];
        }

        if DEBUG {
            println!("IP2: next_point: {}", self.next_point);
        }

        if self.end[self.next_point] == self.neighbor {
            // IP3. Choose the edge.
            if delta_e >= self.slack(self.next_point) / 2 {
                // !!! p. 220. reversed in diss.
                return;
            }
            self.next_point = self.next_pair[self.next_point];
        }

        // IP4.
        self.next_pair[self.pair_point] = self.e as usize;
        self.pair_point = self.e as usize;
        self.next_pair[self.e as usize] = self.next_point;

        // IP5. Update best linking edge.
        if DEBUG {
            println!(
                "IP5: new_base: {} next_delta[new_base]: {} delta_e: {}",
                self.new_base, self.next_delta[self.new_base], delta_e
            );
        }
        if self.next_delta[self.new_base] > delta_e {
            self.next_delta[self.new_base] = delta_e;
        }
    }

    /** Links the unlined vertices inthe path P( end[e], new_base ).
     * Edge e completes a linking path.
     * Invoked by pair.
     * Pre-condition:
     *    new_base == vertex of the new blossom.
     *    new_last == vertex that is currently last on the list of vertices for
     *    new_base's blossom.
     */
    fn link_path(&mut self, mut e: Edge) {
        if DEBUG {
            println!(
                "Link Path e = {} END[e]: {}",
                self.end[self.opp_edge(e as Link) as usize],
                self.end[e]
            );
        }

        // L1. Done?
        /* L1. */
        self.v = self.bend(e);
        while self.v != self.new_base {
            // L2. Link next vertex.
            let u = self.bmate(self.v);
            self.link[u] = self.opp_edge(e as Link);

            if DEBUG {
                println!(" L2: LINK[{}]: {}", u, self.link[u]);
            }

            // L3. Add vertices to blossom list.
            self.next_vertex[self.new_last] = self.v;
            self.next_vertex[self.last_vertex[self.v]] = u;
            self.new_last = self.last_vertex[u];
            let mut i = self.v;

            // L4. Update base.
            loop {
                self.base[i] = self.new_base;
                i = self.next_vertex[i];
                if i == self.dummy_vertex {
                    break;
                }
            }

            // L5. Get next edge.
            e = self.link[self.v] as Edge;

            self.v = self.bend(e);
        }
    }

    /** Merges a subblossom's pair list into a new blossom's pair list.
     * Invoked by pair.
     * Pre-condition:
     *    v is the base of a previously linked subblossom.
     */
    fn merge_pairs(&mut self, v: Vertex) {
        if DEBUG {
            println!("Merge Pairs v = {} last_delta: {}", v, self.last_delta);
        }
        // MP1. Prepare to merge.
        self.next_delta[v] = self.last_delta;

        if DEBUG {
            println!(
                " merge_pairs: v: {} next_delta[v]: {} last_delta: {}",
                v, self.next_delta[v], self.last_delta
            );
        }

        self.pair_point = self.dummy_edge;
        self.f = self.next_edge[v] as Link;
        while self.f != self.dummy_edge as Link {
            // MP2. Prepare to insert.
            self.e = self.f;
            self.neighbor = self.end[self.e as usize];
            self.f = self.next_pair[self.f as usize] as Link;

            // MP3. Insert edge.
            if self.base[self.neighbor] != self.new_base {
                self.insert_pair();
            }
        }
    }

    /**  Processes an edge joining 2 linked vertices. Invoked from W4 of
     * weighted_match.
     * Pre-condition:
     *    v is the base of 1 end of the linking edge.
     * Pair checks whether the edge completes an augmenting path or a pair link
     * path.
     * returns true iff an augmenting path is found.
     */
    fn pair(&mut self) -> bool {
        if DEBUG {
            println!("pair: v: {}", self.v);
        }
        let mut u;
        let mut w;

        // PA1. Prepare to find edge.
        self.e = self.next_edge[self.v] as Link;

        // PA2. Find edge.
        while self.slack(self.e as usize) != 2 * self.delta {
            self.e = self.next_pair[self.e as usize] as Link;
        }

        // PA3. Begin flagging vertices.
        w = self.bend(self.e as usize);
        let bmate_w = self.bmate(w);
        self.link[bmate_w] = -self.e; // Flag bmate(w)

        if DEBUG {
            println!(
                " PA3 LINK[{}]: {} w: {} bmate: {} e: {}",
                self.bmate(w),
                self.link[self.bmate(w)],
                w,
                self.bmate(w),
                self.e
            );
        }

        u = self.bmate(self.v);

        // PA4. Flag vertices.
        while self.link[u] != -self.e {
            // u is NOT FLAGGED
            self.link[u] = -self.e;

            if DEBUG {
                println!(" PA4 LINK[{}]: {} e: {}", u, self.link[u], self.e);
            }

            if self.mate[w] != self.dummy_edge {
                mem::swap(&mut self.v, &mut w);
            }
            self.v = self.blink(self.v);
            u = self.bmate(self.v);
        }

        // PA5. Augmenting path?
        if u == self.dummy_vertex && self.v != w {
            return true; // augmenting path found
        }

        // PA6. Prepare to link vertices.
        self.new_last = self.v;
        self.new_base = self.v;
        self.old_first = self.next_vertex[self.v];

        // PA7. Link vertices.
        self.link_path(self.e as usize);
        self.link_path(self.opp_edge(self.e) as usize);

        // PA8. Finish linking.
        self.next_vertex[self.new_last] = self.old_first;
        if self.last_vertex[self.new_base] == self.new_base {
            self.last_vertex[self.new_base] = self.new_last;
            if DEBUG {
                println!(" PA8 last_vertex[{}]:={}", self.new_base, self.new_last);
            }
        }

        // PA9. Start new pair list.
        self.next_pair[self.dummy_edge] = self.dummy_edge;
        self.merge_pairs(self.new_base);
        self.i = self.next_vertex[self.new_base];
        loop {
            // PA10. Merge subblossom's pair list
            self.merge_pairs(self.i);
            self.i = self.next_vertex[self.last_vertex[self.i]];

            // PA11. Scan subblossom.
            self.scan(self.i, 2 * self.delta - self.slack(self.mate[self.i]));
            self.i = self.next_vertex[self.last_vertex[self.i]];

            // PA12. More blossoms?
            if self.i == self.old_first {
                break;
            }
        }

        // PA14.
        return false;
    }

    /**
     * pointer assigns a pointer link to a vertex. Vertices u & v are the bases
     * of blossoms matched with each other. Edge e joins a vertex in blossom u
     * to a linked vertex.
     *
     * pointer is invoked by weighted_match to link exposed vertices (step W2)
     * and to link unlinked vertices (step W5), and from unpair (steps UP5, UP7)
     * to relink vertices.
     */
    fn pointer(&mut self, u: usize, v: Vertex, e: Link) {
        if DEBUG {
            println!(
                "\nPointer on entry: delta: {} u: {} v: {} e: {} opp_edge(e) = {}",
                self.delta,
                u,
                v,
                e,
                self.opp_edge(e)
            );
            //println!("Pointer: end[opp_edge(e)]" + end[opp_edge(e)]);
            //println!("Pointer: u, v, e = " + u + " " + v + " " + end[opp_edge(e)] + " " + end[e]);
        }
        let mut i;
        let del; // !! Is this declaration correct. Check both i & del.

        if DEBUG {
            println!(
                "\nPointer: delta: {} u: {} v: {} e: {} opp_edge(e) = {}",
                self.delta,
                u,
                v,
                e,
                self.opp_edge(e)
            );
            //println!("Pointer: end[opp_edge(e)]" + end[opp_edge(e)]);
            //println!("Pointer: u, v, e = " + u + " " + v + " " + end[opp_edge(e)] + " " + end[e]);
        }

        // PT1. Reinitialize values that may have changed.
        self.link[u] = -(self.dummy_edge as Link);

        if DEBUG {
            println!(
                "PT1. LINK[{}]: {} dummy_edge: {}",
                u, self.link[u], self.dummy_edge
            );
        }
        self.next_vertex[self.last_vertex[u]] = self.dummy_vertex;
        self.next_vertex[self.last_vertex[v]] = self.dummy_vertex;

        //println!("Pointer: PT2. " + (last_vertex[u] != u ));
        // PT2. Find unpairing value.
        if self.last_vertex[u] != u {
            // u's blossom contains other vertices
            i = self.mate[self.next_vertex[u]];
            //println!("Pointer: true: i: {}", i);
            del = -self.slack(i) / 2;
        } else {
            //println!("Pointer: false: last_delta: " + last_delta);
            del = self.last_delta;
        }
        i = u;

        if DEBUG {
            println!(" PT3. del: {}", del);
        }

        // PT3.
        while i != self.dummy_vertex {
            self.y[i] += del;
            self.next_delta[i] += del;
            if DEBUG {
                println!(
                    " PT3: i: {} next_delta[i]: {} del: {}",
                    i, self.next_delta[i], del
                );
            }
            i = self.next_vertex[i];
        }

        // PT4. Link v & scan.

        if DEBUG {
            println!("POINTER: ?? LINK < 0 ?? LINK: {} v: {}", self.link[v], v);
        }

        if self.link[v] < 0 {
            // v is unlinked.
            self.link[v] = e;

            if DEBUG {
                println!("PT4. LINK[{}]: {} e: {}", v, self.link[v], e);
            }

            self.next_pair[self.dummy_edge] = self.dummy_edge;
            self.scan(v, self.delta);
        } else {
            /* Yes, it looks like this statement can be factored out, and put
             * after if condition, eliminating the else.
             * However, link is a global variable used in scan:
             *
             * I'm not fooling with it!
             */
            self.link[v] = e;

            if DEBUG {
                //println!("PT4.1. LINK[" + v + "]: " + + link[v] + " e: " + e);
            }
        }
    }

    /** Changes the matching along an alternating path.
     * Invoked by weighted_match (W7) to augment the matching, and from unpair
     * (UP2) and unpair_all (UA6) to rematch a blossom.
     *
     * Pre-conditions:
     *    first_mate is the first base vertex on the alternating path.
     *    Edge e is the new matched edge for first_mate.
     */
    fn rematch(&mut self, mut first_mate: Vertex, mut e: Link) {
        if DEBUG {
            println!(
                "rematch: first_mate: {}, end[ opp_edge( e ) ]: {}, end[e]: {}",
                first_mate,
                self.end[self.opp_edge(e) as usize],
                self.end[e as usize]
            );
        }
        // R1. Start rematching.
        self.mate[first_mate] = e as usize;
        self.next_e = -self.link[first_mate];

        // R2. Done?
        while self.next_e != self.dummy_edge as Link {
            // R3. Get next edge.
            e = self.next_e;
            self.f = self.opp_edge(e);
            first_mate = self.bend(e as usize);
            self.second_mate = self.bend(self.f as usize);
            self.next_e = -self.link[first_mate];

            // R4. Relink and rematch.
            self.link[first_mate] = -(self.mate[self.second_mate] as Link);
            self.link[self.second_mate] = -(self.mate[first_mate] as Link);

            if DEBUG {
                println!(
                    concat!(
                        "R4: LINK[{}]: {}",
                        " link[{}]: {} first_mate: {}",
                        " second_mate: {} mate[second_mate]: {}",
                        " mate[fisrtMate]: {}"
                    ),
                    first_mate,
                    self.link[first_mate],
                    self.second_mate,
                    self.link[self.second_mate],
                    first_mate,
                    self.second_mate,
                    self.mate[self.second_mate],
                    self.mate[first_mate]
                );
            }

            self.mate[first_mate] = self.f as usize;
            self.mate[self.second_mate] = e as usize;
        }
    }

    /**
     * scan scans a linked blossom. Vertex x is the base of a blossom that has
     * just been linked by either pointer or pair. del is used to update y.
     * scan is invoked with the list head next_pair[dummy_edge] pointing to the
     * 1st edge on the pair list of base[x].
     */
    fn scan(&mut self, mut x: usize, del: Weight) {
        if DEBUG {
            println!("Scan del= {} x= {}", del, x);
        }

        // SC1. Initialize.
        self.new_base = self.base[x];
        self.stop_scan = self.next_vertex[self.last_vertex[x]];
        while x != self.stop_scan {
            // SC2. Set bounds & initialize for x.
            self.y[x] += del;
            self.next_delta[x] = self.last_delta;

            if DEBUG {
                println!(
                    " SC2: x: {} last_delta: {} next_delta: {}",
                    x, self.last_delta, self.next_delta[x]
                );
            }

            self.pair_point = self.dummy_edge;
            self.e = self.a[x] as Link; // !!! in dissertation: if there are no edges, go to SC7.
            while self.e != 0 {
                // SC3. Find a neighbor.
                self.neighbor = self.end[self.e as usize];
                let u = self.base[self.neighbor];

                // SC4. Pair link edge.
                if DEBUG {
                    println!("Scan: SC4: link[{}]: {}", u, self.link[u]);
                }
                if self.link[u] < 0 {
                    if self.link[self.bmate(u)] < 0 || self.last_vertex[u] != u {
                        let del_e = self.slack(self.e as usize);
                        if self.next_delta[self.neighbor] > del_e {
                            self.next_delta[self.neighbor] = del_e;
                            self.next_edge[self.neighbor] = self.e as usize;

                            if DEBUG {
                                println!(
                                    " SC4.1: neighbor: {} next_delta[neighbor]: {} del_e: {}",
                                    self.neighbor, self.next_delta[self.neighbor], del_e
                                );
                            }
                        }
                    }
                } else {
                    // SC5.
                    if u != self.new_base {
                        if DEBUG {
                            println!("Scan: SC5: u: {} new_base: {}", u, self.new_base);
                        }
                        self.insert_pair();
                    }
                }

                /* SC6. */
                self.e = self.a[self.e as usize] as Link;
            }

            /* SC7. */
            x = self.next_vertex[x];
        }

        // SC8.
        self.next_edge[self.new_base] = self.next_pair[self.dummy_edge];
    }

    /** Updates numerical bounds for linking paths.
     * Invoked by weigtedMatch
     *
     * Pre-condition:
     *    last_delta set to bound on delta for the next search.
     */
    fn set_bounds(&mut self) {
        if DEBUG {
            println!("set_bounds: entered: delta: {}", self.delta);
        }

        // SB1. Examine each vertex.
        for v1 in 1..=self.max_v {
            self.v = v1;
            // SB2. Is vertex a linked base?
            if self.link[self.v] < 0 || self.base[self.v] != self.v {
                // SB8. Update next_delta.
                self.next_delta[self.v] = self.last_delta;

                if DEBUG {
                    println!(
                        " set_bounds: v: {} next_delta[v]: {} last_delta: {}",
                        self.v, self.next_delta[self.v], self.last_delta
                    );
                }
                continue;
            }

            // SB3. Begin processing linked blossom.
            self.link[self.v] = -self.link[self.v];

            if DEBUG {
                println!(" SB3: LINK[{}]: {}", self.v, self.link[self.v]);
            }

            self.i = self.v;

            // SB4. Update y in linked blossom.
            // !! discrepancy: dissertation (do-while); Rothberg (while)
            while self.i != self.dummy_vertex {
                self.y[self.i] -= self.delta;
                self.i = self.next_vertex[self.i];
            }

            // SB5. Is linked blossom matched?
            self.f = self.mate[self.v] as Link;
            if self.f != self.dummy_edge as Link {
                // SB6. Begin processing unlinked blossom.
                self.i = self.bend(self.f as usize);
                let del = self.slack(self.f as usize);

                // SB7. Update y in unlinked blossom.
                // !! discrepancy: dissertation (do-while); Rothberg (while)
                while self.i != self.dummy_vertex {
                    self.y[self.i] -= del;
                    self.i = self.next_vertex[self.i];
                }
            }
            self.next_delta[self.v] = self.last_delta;

            if DEBUG {
                println!(
                    " set_bounds: v: {} next_delta[v]: {} last_delta: {}",
                    self.v, self.next_delta[self.v], self.last_delta
                );
            }
        }
        self.v = self.max_v + 1;
    }

    fn set_up(&mut self) {
        let mut current_edge = self.max_v + 2;
        //println!("set_up: initial current_edge: " + current_edge);
        for i in (1..=self.max_v).rev() {
            for j in (1..i).rev() {
                /* !! in Rothberg, I only understand the SetMatrix function in the
                 * file "term.c".
                 * He seems to treat each matrix entry as a directed arc weight in
                 * a symmetric graph. Thus, he multiplies its value by 2,
                 * representing the undirected symmetric equivalent.
                 *
                 * If I deviate from this, I must also change initialize, which also
                 * refers to the costs matrix.
                 */
                if DEBUG {
                    println!(
                        "set_up: i-1: {} j-1: {} cost: {}",
                        i,
                        j,
                        self.costs[i - 1][j - 1]
                    );
                }
                let cost = 2 * self.costs[i - 1][j - 1];
                self.weight[current_edge - 1] = cost;
                self.weight[current_edge] = cost;
                self.end[current_edge - 1] = i;
                self.end[current_edge] = j;
                self.a[current_edge] = self.a[i];
                self.a[i] = current_edge;
                self.a[current_edge - 1] = self.a[j];
                self.a[j] = current_edge - 1;
                /*
                if DEBUG {
                    println!("set_up: i: " + i + ", j: " + j +
                    ", costs[i-1,j-1]: " + costs[i-1][j-1] + ", current_edge: " + current_edge +
                    "\n\t weight: " + weight[current_edge-1] + " " + weight[current_edge-1] +
                    "\n\t end: " + end[current_edge-1] +" " + end[current_edge-1] +
                    "\n\t a: " + a[current_edge-1] +" " + a[current_edge-1] +
                    "\n\t a[i], a[j]: " + a[i] +" " + a[j]
                 );
                }
                 */
                current_edge += 2;
            }
        }
    }

    /** Unlinks subblossoms in a blossom.
     * Invoked by unpair and unpair_all
     * Pre-conditions:
     *    oldbase is the base of the blossom to be unlinked.
     * unlink preserves the values of the links it undoes, for use by rematch
     * and unpair.
     *
     * unlink sets the array last_edge, for use by unpair and unpair_all.
     */
    fn unlink(&mut self, old_base: usize) {
        if DEBUG {
            println!("unlink: old_base: {}", old_base);
        }

        // UL1. Prepare to unlink paths.
        self.i = self.next_vertex[old_base];
        self.new_base = self.next_vertex[old_base];
        self.next_base = self.next_vertex[self.last_vertex[self.new_base]];
        self.e = self.link[self.next_base];

        // Loop is executed twice, for the 2 paths containing the subblossom.
        for j in 1..=2 {
            loop {
                // UL2. Get next path edge.
                if DEBUG {
                    println!("UL2. j: {}", j);
                }
                self.nxt_edge = self.opp_edge(self.link[self.new_base]);

                for _ in 1..=2 {
                    // UL3. Unlink blossom base.
                    self.link[self.new_base] = -self.link[self.new_base];

                    if DEBUG {
                        println!("UL3. LINK[{}]: {}", self.new_base, self.link[self.new_base]);
                    }

                    // UL4. Update base array.
                    loop {
                        self.base[self.i] = self.new_base;
                        self.i = self.next_vertex[self.i];
                        if self.i == self.next_base {
                            break;
                        }
                    }

                    // UL5. Get next vertex.
                    self.new_base = self.next_base;
                    self.next_base = self.next_vertex[self.last_vertex[self.new_base]];
                }

                // UL6. More vertices?
                if self.link[self.next_base] != self.nxt_edge {
                    break;
                }
            }

            // UL7. End of path.
            if j == 1 {
                self.last_edge[1] = self.nxt_edge;
                self.nxt_edge = self.opp_edge(self.e);
                if self.link[self.next_base] == self.nxt_edge {
                    if DEBUG {
                        println!("UL7*. Going to UL2.");
                    }
                    continue; // check the control flow logic.
                }
            }
            break;
        }
        self.last_edge[2] = self.nxt_edge;

        // UL8. Update blossom list.
        if self.base[self.last_vertex[old_base]] == old_base {
            self.next_vertex[old_base] = self.new_base;
        } else {
            self.next_vertex[old_base] = self.dummy_vertex;
            self.last_vertex[old_base] = old_base;
        }
    }

    /** Undoes a blossom by unlinking, rematching, and relinking subblossoms.
     * Invoked by weighted_match
     * Pre-conditions:
     *    old_base == an unlinked vertex, the base of the blossom to be undone.
     *    old_mate == a linked vertex, the base of the blossom matched to old_base
     *
     * It uses a local variable newbase.
     */
    fn unpair(&mut self, old_base: usize, old_mate: Vertex) {
        if DEBUG {
            println!("Unpair old_base: {}, old_mate: {}", old_base, old_mate);
        }

        // UP1. Unlink vertices.
        self.unlink(old_base);

        // UP2. Rematch a path.
        let newbase = self.bmate(old_mate);
        if newbase != old_base {
            self.link[old_base] = -(self.dummy_edge as Link);
            self.rematch(newbase, self.mate[old_base] as isize);
            self.link[self.second_mate] = if self.f == self.last_edge[1] {
                -self.last_edge[2]
            } else {
                -self.last_edge[1]
            };
        }

        // UP3. Examine the linking edge.
        let mut e = self.link[old_mate];
        let mut u = self.bend(self.opp_edge(e) as usize);
        if u == newbase {
            // UP7. Relink oldmate.
            self.pointer(newbase, old_mate, e);
            return;
        }
        let bmate_u = self.bmate(u);
        self.link[bmate_u] = -e;
        // UP4. missing from dissertation.
        loop {
            // UP5. Relink a vertex
            e = -self.link[u];
            self.v = self.bmate(u);
            self.pointer(u, self.v, -self.link[self.v]);

            // UP6. Get next blossom.
            u = self.bend(e as usize);
            if u == newbase {
                break;
            }
        }
        e = self.opp_edge(e);

        // UP7. Relink oldmate
        self.pointer(newbase, old_mate, e);
    }

    /** Undoes all the blossoms, rematching them to get the final matching.
     * Invoked by weighted_match.
     */
    fn unpair_all(&mut self) {
        // UA1. Unpair each blossom.
        for v1 in 1..=self.max_v {
            self.v = v1;
            if self.base[self.v] != self.v || self.last_vertex[self.v] == self.v {
                continue;
            }

            // UA2. Prepare to unpair.
            self.next_u = self.v;
            self.next_vertex[self.last_vertex[self.next_u]] = self.dummy_vertex;

            loop {
                // UA3. Get next blossom to unpair.
                let u = self.next_u;
                self.next_u = self.next_vertex[self.next_u];

                // UA4. Unlink a blossom.
                self.unlink(u);
                if self.last_vertex[u] != u {
                    // UA5. List subblossoms to unpair.
                    self.f = if self.last_edge[2] == self.opp_edge(self.e) {
                        self.last_edge[1]
                    } else {
                        self.last_edge[2]
                    };
                    let bend_f = self.bend(self.f as usize);
                    self.next_vertex[self.last_vertex[bend_f]] = u;
                    if DEBUG {
                        println!("UA5. f: {}", self.f);
                    }
                }

                // UA6. Rematch blossom.
                self.new_base = self.bmate(self.bmate(u));
                if self.new_base != self.dummy_vertex && self.new_base != u {
                    self.link[u] = -(self.dummy_edge as Link);
                    self.rematch(self.new_base, self.mate[u] as Link);
                }

                // UA7. Find next blossom to unpair.
                while self.last_vertex[self.next_u] == self.next_u
                    && self.next_u != self.dummy_vertex
                {
                    self.next_u = self.next_vertex[self.next_u];
                }
                if self.last_vertex[self.next_u] == self.next_u && self.next_u == self.dummy_vertex
                {
                    break;
                }
            }
        }
    }
}

pub fn weightedmatch(costs: Vec<Vec<Weight>>, minimize_weight: bool) -> Vec<Edge> {
    let mut m = WeightedMatch::new(costs);
    m.weighted_match(minimize_weight)
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
