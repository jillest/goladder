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
 * It is unclear to me why cost values are doubled in setUp() and intialize(). I
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

    V: usize,
    E: usize,
    dummyVertex: Vertex,
    dummyEdge: Edge,

    a: Vec<usize>, // adjacency list
    end: Vec<usize>,
    mate: Vec<Edge>,
    weight: Vec<Weight>,

    base: Vec<usize>,
    lastEdge: [Link; 3], // Used by methods that undo blossoms.
    lastVertex: Vec<usize>,
    link: Vec<Link>,
    nextDelta: Vec<Weight>,
    nextEdge: Vec<Edge>,
    nextPair: Vec<Edge>,
    nextVertex: Vec<Edge>,
    y: Vec<Weight>,

    delta: Weight,
    lastDelta: Weight,
    newBase: usize,
    nextBase: usize,
    stopScan: usize,
    pairPoint: usize,
    neighbor: usize,
    newLast: usize,
    nextPoint: Vertex,
    oldFirst: usize,
    secondMate: usize,
    f: Link,
    nxtEdge: Link,
    nextE: Link,
    nextU: usize,

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
     * if ( minimizeWeight ) <br>
     * &nbsp;&nbsp;&nbsp;&nbsp; performs a minimum cost maximum matching;<br>
     * else<br>
     *    performs a maximum cost maximum matching.
     * @param minimizeWeight if ( minimizeWeight )
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
    fn weightedMatch(&mut self, minimizeWeight: bool) -> Vec<Edge> {
        if DEBUG {
            println!("weightedMatch: input costs matrix:");
            for (i, row) in self.costs.iter().enumerate() {
                print!(" Row {}:", i);
                for val in row.iter().skip(i + 1) {
                    print!(" {}", val);
                }
                println!("");
            }
        }

        let mut loopNum = 1;

        // W0. Input.
        self.input();

        // W1. Initialize.
        self.initialize(minimizeWeight);

        loop {
            if DEBUG {
                println!("\n *** A U G M E N T {}", loopNum);
                loopNum += 1;
            }
            // W2. Start a new search.
            self.delta = 0;
            for v1 in 1..=self.V {
                self.v = v1;
                if self.mate[self.v] == self.dummyEdge {
                    // Link all exposed vertices.
                    self.pointer(self.dummyVertex, self.v, self.dummyEdge as Link);
                }
            }
            self.v = self.V + 1;

            if DEBUG {
                for q in 1..=self.V + 1 {
                    println!(
                        concat!(
                            "W2: i: {}",
                            ", mate: {}",
                            ", nextEdge: {}",
                            ", nextVertex: {}",
                            ", link: {}",
                            ", base: {}",
                            ", lastVertex: {}",
                            ", y: {}",
                            ", nextDelta: {}",
                            ", lastDelta: {}"
                        ),
                        q,
                        self.mate[q],
                        self.nextEdge[q],
                        self.nextVertex[q],
                        self.link[q],
                        self.base[q],
                        self.lastVertex[q],
                        self.y[q],
                        self.nextDelta[q],
                        self.lastDelta
                    );
                }
            }

            // W3. Get next edge.
            loop {
                self.i = 1;
                for j in 2..=self.V {
                    /* !!! Dissertation, p. 213, it is nextDelta[i] < nextDelta[j]
                     * When I make it <, the routine seems to do nothing.
                     */
                    if self.nextDelta[self.i] > self.nextDelta[j] {
                        self.i = j;
                    }
                }

                // delta is the minimum slack in the next edge.
                self.delta = self.nextDelta[self.i];

                if DEBUG {
                    println!("\nW3: i: {} delta: {}", self.i, self.delta);
                }

                if self.delta == self.lastDelta {
                    if DEBUG {
                        println!("\nW8: delta: {} lastDelta: {}", self.delta, self.lastDelta);
                    }
                    // W8. Undo blossoms.
                    self.setBounds();
                    self.unpairAll();
                    for i in 1..=self.V {
                        self.mate[i] = self.end[self.mate[i]];
                        if self.mate[i] == self.dummyVertex {
                            self.mate[i] = UNMATCHED;
                        }
                    }
                    self.mate.truncate(self.V + 1);

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
                        self.pointer(self.v, w, self.oppEdge(self.nextEdge[self.i] as Link));
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
            self.lastDelta -= self.delta;
            self.setBounds();
            let g = self.oppEdge(self.e);
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

    fn oppEdge(&self, e: isize) -> isize {
        if (e - self.V as isize) % 2 == 0 {
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
        return self.y[self.end[e]] + self.y[self.end[self.oppEdge(e as Link) as usize]]
            - self.weight[e];
    }
    //
    // End 5 simple functions

    fn initialize(&mut self, minimizeWeight: bool) {
        // initialize basic data structures
        self.setUp();

        if DEBUG {
            for q in 0..self.V + 2 * self.E + 2 {
                println!(
                    "initialize: i: {}, a: {} end: {} weight: {}",
                    q, self.a[q], self.end[q], self.weight[q]
                );
            }
        }

        self.dummyVertex = self.V + 1;
        self.dummyEdge = self.V + 2 * self.E + 1;
        self.end[self.dummyEdge] = self.dummyVertex;

        if DEBUG {
            println!(
                "initialize: dummyVertex: {} dummyEdge: {} oppEdge(dummyEdge): {}",
                self.dummyVertex,
                self.dummyEdge,
                self.oppEdge(self.dummyEdge as Link)
            );
        }

        let mut maxWeight = Weight::min_value();
        let mut minWeight = Weight::max_value();
        for i in 0..self.V {
            for j in i + 1..self.V {
                let cost = 2 * self.costs[i][j];
                if cost > maxWeight {
                    maxWeight = cost;
                }
                if cost < minWeight {
                    minWeight = cost;
                }
            }
        }

        if DEBUG {
            println!(
                "initialize: minWeight: {}, maxWeight: {}",
                minWeight, maxWeight
            );
        }

        // If minimize costs, invert weights
        if minimizeWeight {
            if self.V % 2 != 0 {
                panic!("|V| must be even for a minimum cost maximum matching.");
            }
            maxWeight += 2; // Don't want all 0 weight
            for i in self.V + 1..=self.V + 2 * self.E {
                self.weight[i] = maxWeight - self.weight[i];
                //println!("initialize: inverted weight[" + i + "]: " +
                //weight[i]);
            }
            maxWeight = maxWeight - minWeight;
        }

        self.lastDelta = maxWeight / 2;
        if DEBUG {
            println!(
                "initialize: minWeight: {} maxWeight: {} lastDelta: {}",
                minWeight, maxWeight, self.lastDelta
            );
        }

        let allocationSize = self.V + 2;
        self.mate = vec![0; allocationSize];
        self.link = vec![0; allocationSize];
        self.base = vec![0; allocationSize];
        self.nextVertex = vec![0; allocationSize];
        self.lastVertex = vec![0; allocationSize];
        self.y = vec![0; allocationSize];
        self.nextDelta = vec![0; allocationSize];
        self.nextEdge = vec![0; allocationSize];

        let allocationSize = self.V + 2 * self.E + 2;
        self.nextPair = vec![0; allocationSize];

        for i in 1..=self.V + 1 {
            self.mate[i] = self.dummyEdge;
            self.nextEdge[i] = self.dummyEdge;
            self.nextVertex[i] = 0;
            self.link[i] = -(self.dummyEdge as Link);
            self.base[i] = i;
            self.lastVertex[i] = i;
            self.y[i] = self.lastDelta;
            self.nextDelta[i] = self.lastDelta;

            if DEBUG {
                println!(
                    concat!(
                        "initialize: v: {}, i: {}",
                        ", mate: {}",
                        ", nextEdge: {}",
                        ", nextVertex: {}",
                        ", link: {}",
                        ", base: {}",
                        ", lastVertex: {}",
                        ", y: {}",
                        ", nextDelta: {}",
                        ", lastDelta: {}"
                    ),
                    self.v,
                    i,
                    self.mate[i],
                    self.nextEdge[i],
                    self.nextVertex[i],
                    self.link[i],
                    self.base[i],
                    self.lastVertex[i],
                    self.y[i],
                    self.nextDelta[i],
                    self.lastDelta
                );
            }
        }
        self.i = self.V + 2;
        //println!("initialize: complete.");
    }

    fn input(&mut self) {
        self.V = self.costs.len();
        self.E = self.V * (self.V - 1) / 2;

        let allocationSize = self.V + 2 * self.E + 2;
        self.a = vec![0; allocationSize];
        self.end = vec![0; allocationSize];
        self.weight = vec![0; allocationSize];

        if DEBUG {
            println!(
                "input: V: {}, E: {}, allocationSize: {}",
                self.V, self.E, allocationSize
            );
        }
    }

    /** Updates a blossom's pair list, possibly inserting a new edge.
     * It is invoked by scan and mergePairs.
     * It is invoked with global int e set to the edge to be inserted, neighbor
     * set to the end vertex of e, and pairPoint pointing to the next pair to be
     * examined in the pair list.
     */
    fn insertPair(&mut self) {
        if DEBUG {
            println!(
                "Insert Pair e: {} {}- {}",
                self.e,
                self.end[self.oppEdge(self.e) as usize],
                self.end[self.e as usize]
            );
        }

        // IP1. Prepare to insert.
        let deltaE = self.slack(self.e as Edge) / 2;

        if DEBUG {
            println!("IP1: deltaE: {}", deltaE);
        }

        self.nextPoint = self.nextPair[self.pairPoint];

        // IP2. Fint insertion point.
        while self.end[self.nextPoint] < self.neighbor {
            self.pairPoint = self.nextPoint;
            self.nextPoint = self.nextPair[self.nextPoint];
        }

        if DEBUG {
            println!("IP2: nextPoint: {}", self.nextPoint);
        }

        if self.end[self.nextPoint] == self.neighbor {
            // IP3. Choose the edge.
            if deltaE >= self.slack(self.nextPoint) / 2 {
                // !!! p. 220. reversed in diss.
                return;
            }
            self.nextPoint = self.nextPair[self.nextPoint];
        }

        // IP4.
        self.nextPair[self.pairPoint] = self.e as usize;
        self.pairPoint = self.e as usize;
        self.nextPair[self.e as usize] = self.nextPoint;

        // IP5. Update best linking edge.
        if DEBUG {
            println!(
                "IP5: newBase: {} nextDelta[newBase]: {} deltaE: {}",
                self.newBase, self.nextDelta[self.newBase], deltaE
            );
        }
        if self.nextDelta[self.newBase] > deltaE {
            self.nextDelta[self.newBase] = deltaE;
        }
    }

    /** Links the unlined vertices inthe path P( end[e], newBase ).
     * Edge e completes a linking path.
     * Invoked by pair.
     * Pre-condition:
     *    newBase == vertex of the new blossom.
     *    newLast == vertex that is currently last on the list of vertices for
     *    newBase's blossom.
     */
    fn linkPath(&mut self, mut e: Edge) {
        if DEBUG {
            println!(
                "Link Path e = {} END[e]: {}",
                self.end[self.oppEdge(e as Link) as usize],
                self.end[e]
            );
        }

        // L1. Done?
        /* L1. */
        self.v = self.bend(e);
        while self.v != self.newBase {
            // L2. Link next vertex.
            let u = self.bmate(self.v);
            self.link[u] = self.oppEdge(e as Link);

            if DEBUG {
                println!(" L2: LINK[{}]: {}", u, self.link[u]);
            }

            // L3. Add vertices to blossom list.
            self.nextVertex[self.newLast] = self.v;
            self.nextVertex[self.lastVertex[self.v]] = u;
            self.newLast = self.lastVertex[u];
            let mut i = self.v;

            // L4. Update base.
            loop {
                self.base[i] = self.newBase;
                i = self.nextVertex[i];
                if i == self.dummyVertex {
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
    fn mergePairs(&mut self, v: Vertex) {
        if DEBUG {
            println!("Merge Pairs v = {} lastDelta: {}", v, self.lastDelta);
        }
        // MP1. Prepare to merge.
        self.nextDelta[v] = self.lastDelta;

        if DEBUG {
            println!(
                " mergePairs: v: {} nextDelta[v]: {} lastDelta: {}",
                v, self.nextDelta[v], self.lastDelta
            );
        }

        self.pairPoint = self.dummyEdge;
        self.f = self.nextEdge[v] as Link;
        while self.f != self.dummyEdge as Link {
            // MP2. Prepare to insert.
            self.e = self.f;
            self.neighbor = self.end[self.e as usize];
            self.f = self.nextPair[self.f as usize] as Link;

            // MP3. Insert edge.
            if self.base[self.neighbor] != self.newBase {
                self.insertPair();
            }
        }
    }

    /**  Processes an edge joining 2 linked vertices. Invoked from W4 of
     * weightedMatch.
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
        self.e = self.nextEdge[self.v] as Link;

        // PA2. Find edge.
        while self.slack(self.e as usize) != 2 * self.delta {
            self.e = self.nextPair[self.e as usize] as Link;
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

            if self.mate[w] != self.dummyEdge {
                mem::swap(&mut self.v, &mut w);
            }
            self.v = self.blink(self.v);
            u = self.bmate(self.v);
        }

        // PA5. Augmenting path?
        if u == self.dummyVertex && self.v != w {
            return true; // augmenting path found
        }

        // PA6. Prepare to link vertices.
        self.newLast = self.v;
        self.newBase = self.v;
        self.oldFirst = self.nextVertex[self.v];

        // PA7. Link vertices.
        self.linkPath(self.e as usize);
        self.linkPath(self.oppEdge(self.e) as usize);

        // PA8. Finish linking.
        self.nextVertex[self.newLast] = self.oldFirst;
        if self.lastVertex[self.newBase] == self.newBase {
            self.lastVertex[self.newBase] = self.newLast;
            if DEBUG {
                println!(" PA8 lastVertex[{}]:={}", self.newBase, self.newLast);
            }
        }

        // PA9. Start new pair list.
        self.nextPair[self.dummyEdge] = self.dummyEdge;
        self.mergePairs(self.newBase);
        self.i = self.nextVertex[self.newBase];
        loop {
            // PA10. Merge subblossom's pair list
            self.mergePairs(self.i);
            self.i = self.nextVertex[self.lastVertex[self.i]];

            // PA11. Scan subblossom.
            self.scan(self.i, 2 * self.delta - self.slack(self.mate[self.i]));
            self.i = self.nextVertex[self.lastVertex[self.i]];

            // PA12. More blossoms?
            if self.i == self.oldFirst {
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
     * pointer is invoked by weightedMatch to link exposed vertices (step W2)
     * and to link unlinked vertices (step W5), and from unpair (steps UP5, UP7)
     * to relink vertices.
     */
    fn pointer(&mut self, u: usize, v: Vertex, e: Link) {
        if DEBUG {
            println!(
                "\nPointer on entry: delta: {} u: {} v: {} e: {} oppEdge(e) = {}",
                self.delta,
                u,
                v,
                e,
                self.oppEdge(e)
            );
            //println!("Pointer: end[oppEdge(e)]" + end[oppEdge(e)]);
            //println!("Pointer: u, v, e = " + u + " " + v + " " + end[oppEdge(e)] + " " + end[e]);
        }
        let mut i;
        let del; // !! Is this declaration correct. Check both i & del.

        if DEBUG {
            println!(
                "\nPointer: delta: {} u: {} v: {} e: {} oppEdge(e) = {}",
                self.delta,
                u,
                v,
                e,
                self.oppEdge(e)
            );
            //println!("Pointer: end[oppEdge(e)]" + end[oppEdge(e)]);
            //println!("Pointer: u, v, e = " + u + " " + v + " " + end[oppEdge(e)] + " " + end[e]);
        }

        // PT1. Reinitialize values that may have changed.
        self.link[u] = -(self.dummyEdge as Link);

        if DEBUG {
            println!(
                "PT1. LINK[{}]: {} dummyEdge: {}",
                u, self.link[u], self.dummyEdge
            );
        }
        self.nextVertex[self.lastVertex[u]] = self.dummyVertex;
        self.nextVertex[self.lastVertex[v]] = self.dummyVertex;

        //println!("Pointer: PT2. " + (lastVertex[u] != u ));
        // PT2. Find unpairing value.
        if self.lastVertex[u] != u {
            // u's blossom contains other vertices
            i = self.mate[self.nextVertex[u]];
            //println!("Pointer: true: i: {}", i);
            del = -self.slack(i) / 2;
        } else {
            //println!("Pointer: false: lastDelta: " + lastDelta);
            del = self.lastDelta;
        }
        i = u;

        if DEBUG {
            println!(" PT3. del: {}", del);
        }

        // PT3.
        while i != self.dummyVertex {
            self.y[i] += del;
            self.nextDelta[i] += del;
            if DEBUG {
                println!(
                    " PT3: i: {} nextDelta[i]: {} del: {}",
                    i, self.nextDelta[i], del
                );
            }
            i = self.nextVertex[i];
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

            self.nextPair[self.dummyEdge] = self.dummyEdge;
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
     * Invoked by weightedMatch (W7) to augment the matching, and from unpair
     * (UP2) and unpairAll (UA6) to rematch a blossom.
     *
     * Pre-conditions:
     *    firstMate is the first base vertex on the alternating path.
     *    Edge e is the new matched edge for firstMate.
     */
    fn rematch(&mut self, mut firstMate: Vertex, mut e: Link) {
        if DEBUG {
            println!(
                "rematch: firstMate: {}, end[ oppEdge( e ) ]: {}, end[e]: {}",
                firstMate,
                self.end[self.oppEdge(e) as usize],
                self.end[e as usize]
            );
        }
        // R1. Start rematching.
        self.mate[firstMate] = e as usize;
        self.nextE = -self.link[firstMate];

        // R2. Done?
        while self.nextE != self.dummyEdge as Link {
            // R3. Get next edge.
            e = self.nextE;
            self.f = self.oppEdge(e);
            firstMate = self.bend(e as usize);
            self.secondMate = self.bend(self.f as usize);
            self.nextE = -self.link[firstMate];

            // R4. Relink and rematch.
            self.link[firstMate] = -(self.mate[self.secondMate] as Link);
            self.link[self.secondMate] = -(self.mate[firstMate] as Link);

            if DEBUG {
                println!(
                    concat!(
                        "R4: LINK[{}]: {}",
                        " link[{}]: {} firstMate: {}",
                        " secondMate: {} mate[secondMate]: {}",
                        " mate[fisrtMate]: {}"
                    ),
                    firstMate,
                    self.link[firstMate],
                    self.secondMate,
                    self.link[self.secondMate],
                    firstMate,
                    self.secondMate,
                    self.mate[self.secondMate],
                    self.mate[firstMate]
                );
            }

            self.mate[firstMate] = self.f as usize;
            self.mate[self.secondMate] = e as usize;
        }
    }

    /**
     * scan scans a linked blossom. Vertex x is the base of a blossom that has
     * just been linked by either pointer or pair. del is used to update y.
     * scan is invoked with the list head nextPair[dummyEdge] pointing to the
     * 1st edge on the pair list of base[x].
     */
    fn scan(&mut self, mut x: usize, del: Weight) {
        if DEBUG {
            println!("Scan del= {} x= {}", del, x);
        }

        // SC1. Initialize.
        self.newBase = self.base[x];
        self.stopScan = self.nextVertex[self.lastVertex[x]];
        while x != self.stopScan {
            // SC2. Set bounds & initialize for x.
            self.y[x] += del;
            self.nextDelta[x] = self.lastDelta;

            if DEBUG {
                println!(
                    " SC2: x: {} lastDelta: {} nextDelta: {}",
                    x, self.lastDelta, self.nextDelta[x]
                );
            }

            self.pairPoint = self.dummyEdge;
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
                    if self.link[self.bmate(u)] < 0 || self.lastVertex[u] != u {
                        let delE = self.slack(self.e as usize);
                        if self.nextDelta[self.neighbor] > delE {
                            self.nextDelta[self.neighbor] = delE;
                            self.nextEdge[self.neighbor] = self.e as usize;

                            if DEBUG {
                                println!(
                                    " SC4.1: neighbor: {} nextDelta[neighbor]: {} delE: {}",
                                    self.neighbor, self.nextDelta[self.neighbor], delE
                                );
                            }
                        }
                    }
                } else {
                    // SC5.
                    if u != self.newBase {
                        if DEBUG {
                            println!("Scan: SC5: u: {} newBase: {}", u, self.newBase);
                        }
                        self.insertPair();
                    }
                }

                /* SC6. */
                self.e = self.a[self.e as usize] as Link;
            }

            /* SC7. */
            x = self.nextVertex[x];
        }

        // SC8.
        self.nextEdge[self.newBase] = self.nextPair[self.dummyEdge];
    }

    /** Updates numerical bounds for linking paths.
     * Invoked by weigtedMatch
     *
     * Pre-condition:
     *    lastDelta set to bound on delta for the next search.
     */
    fn setBounds(&mut self) {
        if DEBUG {
            println!("setBounds: entered: delta: {}", self.delta);
        }

        // SB1. Examine each vertex.
        for v1 in 1..=self.V {
            self.v = v1;
            // SB2. Is vertex a linked base?
            if self.link[self.v] < 0 || self.base[self.v] != self.v {
                // SB8. Update nextDelta.
                self.nextDelta[self.v] = self.lastDelta;

                if DEBUG {
                    println!(
                        " setBounds: v: {} nextDelta[v]: {} lastDelta: {}",
                        self.v, self.nextDelta[self.v], self.lastDelta
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
            while self.i != self.dummyVertex {
                self.y[self.i] -= self.delta;
                self.i = self.nextVertex[self.i];
            }

            // SB5. Is linked blossom matched?
            self.f = self.mate[self.v] as Link;
            if self.f != self.dummyEdge as Link {
                // SB6. Begin processing unlinked blossom.
                self.i = self.bend(self.f as usize);
                let del = self.slack(self.f as usize);

                // SB7. Update y in unlinked blossom.
                // !! discrepancy: dissertation (do-while); Rothberg (while)
                while self.i != self.dummyVertex {
                    self.y[self.i] -= del;
                    self.i = self.nextVertex[self.i];
                }
            }
            self.nextDelta[self.v] = self.lastDelta;

            if DEBUG {
                println!(
                    " setBounds: v: {} nextDelta[v]: {} lastDelta: {}",
                    self.v, self.nextDelta[self.v], self.lastDelta
                );
            }
        }
        self.v = self.V + 1;
    }

    fn setUp(&mut self) {
        let mut currentEdge = self.V + 2;
        //println!("setUp: initial currentEdge: " + currentEdge);
        for i in (1..=self.V).rev() {
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
                        "setUp: i-1: {} j-1: {} cost: {}",
                        i,
                        j,
                        self.costs[i - 1][j - 1]
                    );
                }
                let cost = 2 * self.costs[i - 1][j - 1];
                self.weight[currentEdge - 1] = cost;
                self.weight[currentEdge] = cost;
                self.end[currentEdge - 1] = i;
                self.end[currentEdge] = j;
                self.a[currentEdge] = self.a[i];
                self.a[i] = currentEdge;
                self.a[currentEdge - 1] = self.a[j];
                self.a[j] = currentEdge - 1;
                /*
                if DEBUG {
                    println!("setUp: i: " + i + ", j: " + j +
                    ", costs[i-1,j-1]: " + costs[i-1][j-1] + ", currentEdge: " + currentEdge +
                    "\n\t weight: " + weight[currentEdge-1] + " " + weight[currentEdge-1] +
                    "\n\t end: " + end[currentEdge-1] +" " + end[currentEdge-1] +
                    "\n\t a: " + a[currentEdge-1] +" " + a[currentEdge-1] +
                    "\n\t a[i], a[j]: " + a[i] +" " + a[j]
                 );
                }
                 */
                currentEdge += 2;
            }
        }
    }

    /** Unlinks subblossoms in a blossom.
     * Invoked by unpair and unpairAll
     * Pre-conditions:
     *    oldbase is the base of the blossom to be unlinked.
     * unlink preserves the values of the links it undoes, for use by rematch
     * and unpair.
     *
     * unlink sets the array lastEdge, for use by unpair and unpairAll.
     */
    fn unlink(&mut self, oldBase: usize) {
        if DEBUG {
            println!("unlink: oldBase: {}", oldBase);
        }

        // UL1. Prepare to unlink paths.
        self.i = self.nextVertex[oldBase];
        self.newBase = self.nextVertex[oldBase];
        self.nextBase = self.nextVertex[self.lastVertex[self.newBase]];
        self.e = self.link[self.nextBase];

        // Loop is executed twice, for the 2 paths containing the subblossom.
        for j in 1..=2 {
            loop {
                // UL2. Get next path edge.
                if DEBUG {
                    println!("UL2. j: {}", j);
                }
                self.nxtEdge = self.oppEdge(self.link[self.newBase]);

                for _ in 1..=2 {
                    // UL3. Unlink blossom base.
                    self.link[self.newBase] = -self.link[self.newBase];

                    if DEBUG {
                        println!("UL3. LINK[{}]: {}", self.newBase, self.link[self.newBase]);
                    }

                    // UL4. Update base array.
                    loop {
                        self.base[self.i] = self.newBase;
                        self.i = self.nextVertex[self.i];
                        if self.i == self.nextBase {
                            break;
                        }
                    }

                    // UL5. Get next vertex.
                    self.newBase = self.nextBase;
                    self.nextBase = self.nextVertex[self.lastVertex[self.newBase]];
                }

                // UL6. More vertices?
                if self.link[self.nextBase] != self.nxtEdge {
                    break;
                }
            }

            // UL7. End of path.
            if j == 1 {
                self.lastEdge[1] = self.nxtEdge;
                self.nxtEdge = self.oppEdge(self.e);
                if self.link[self.nextBase] == self.nxtEdge {
                    if DEBUG {
                        println!("UL7*. Going to UL2.");
                    }
                    continue; // check the control flow logic.
                }
            }
            break;
        }
        self.lastEdge[2] = self.nxtEdge;

        // UL8. Update blossom list.
        if self.base[self.lastVertex[oldBase]] == oldBase {
            self.nextVertex[oldBase] = self.newBase;
        } else {
            self.nextVertex[oldBase] = self.dummyVertex;
            self.lastVertex[oldBase] = oldBase;
        }
    }

    /** Undoes a blossom by unlinking, rematching, and relinking subblossoms.
     * Invoked by weightedMatch
     * Pre-conditions:
     *    oldBase == an unlinked vertex, the base of the blossom to be undone.
     *    oldMate == a linked vertex, the base of the blossom matched to oldBase
     *
     * It uses a local variable newbase.
     */
    fn unpair(&mut self, oldBase: usize, oldMate: Vertex) {
        if DEBUG {
            println!("Unpair oldBase: {}, oldMate: {}", oldBase, oldMate);
        }

        // UP1. Unlink vertices.
        self.unlink(oldBase);

        // UP2. Rematch a path.
        let newbase = self.bmate(oldMate);
        if newbase != oldBase {
            self.link[oldBase] = -(self.dummyEdge as Link);
            self.rematch(newbase, self.mate[oldBase] as isize);
            self.link[self.secondMate] = if self.f == self.lastEdge[1] {
                -self.lastEdge[2]
            } else {
                -self.lastEdge[1]
            };
        }

        // UP3. Examine the linking edge.
        let mut e = self.link[oldMate];
        let mut u = self.bend(self.oppEdge(e) as usize);
        if u == newbase {
            // UP7. Relink oldmate.
            self.pointer(newbase, oldMate, e);
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
        e = self.oppEdge(e);

        // UP7. Relink oldmate
        self.pointer(newbase, oldMate, e);
    }

    /** Undoes all the blossoms, rematching them to get the final matching.
     * Invoked by weightedMatch.
     */
    fn unpairAll(&mut self) {
        // UA1. Unpair each blossom.
        for v1 in 1..=self.V {
            self.v = v1;
            if self.base[self.v] != self.v || self.lastVertex[self.v] == self.v {
                continue;
            }

            // UA2. Prepare to unpair.
            self.nextU = self.v;
            self.nextVertex[self.lastVertex[self.nextU]] = self.dummyVertex;

            loop {
                // UA3. Get next blossom to unpair.
                let u = self.nextU;
                self.nextU = self.nextVertex[self.nextU];

                // UA4. Unlink a blossom.
                self.unlink(u);
                if self.lastVertex[u] != u {
                    // UA5. List subblossoms to unpair.
                    self.f = if self.lastEdge[2] == self.oppEdge(self.e) {
                        self.lastEdge[1]
                    } else {
                        self.lastEdge[2]
                    };
                    let bend_f = self.bend(self.f as usize);
                    self.nextVertex[self.lastVertex[bend_f]] = u;
                    if DEBUG {
                        println!("UA5. f: {}", self.f);
                    }
                }

                // UA6. Rematch blossom.
                self.newBase = self.bmate(self.bmate(u));
                if self.newBase != self.dummyVertex && self.newBase != u {
                    self.link[u] = -(self.dummyEdge as Link);
                    self.rematch(self.newBase, self.mate[u] as Link);
                }

                // UA7. Find next blossom to unpair.
                while self.lastVertex[self.nextU] == self.nextU && self.nextU != self.dummyVertex {
                    self.nextU = self.nextVertex[self.nextU];
                }
                if self.lastVertex[self.nextU] == self.nextU && self.nextU == self.dummyVertex {
                    break;
                }
            }
        }
    }

    fn getMatched(mates: Vec<Edge>) -> Vec<Edge> {
        /* WeightedMatch.weightedMatch returns mates, indexed and valued
         * 1, ..., V. Shift the index to 0, ... , V-1 and put the values in
         * this range too (i.e., decrement them).
         */
        mates.into_iter().skip(1).map(|e| e - 1).collect()
    }
}

pub fn weightedmatch(costs: Vec<Vec<Weight>>, minimize_weight: bool) -> Vec<Edge> {
    let mut m = WeightedMatch::new(costs);
    m.weightedMatch(minimize_weight)
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
