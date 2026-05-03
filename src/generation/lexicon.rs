use std::collections::HashMap;
use std::fs;

const PARTICLE_SIZE: usize = 2;

type Particle = [char; PARTICLE_SIZE];

#[derive(Debug)]
struct LexiconEdge {
    to: Particle,
    weight: u32,
}

#[derive(Debug)]
struct LexiconNode {
    text: Particle,
    num_connections: u32,
    edges: Vec<LexiconEdge>,
}

impl LexiconNode {
    fn new(text: Particle) -> Self {
        Self {
            text,
            num_connections: 0,
            edges: vec![],
        }
    }

    fn add_edge(&mut self, to: Particle) {
        self.num_connections += 1;
        if let Some(edge) = self.edges.iter_mut().find(|e| e.to == to) {
            edge.weight += 1;
        } else {
            self.edges.push(LexiconEdge { to, weight: 1 });
        }
        // Keep sorted descending by weight
        self.edges.sort_by(|a, b| b.weight.cmp(&a.weight));
    }

    fn create_edge(&mut self, to: Particle, weight: u32) {
        self.num_connections += weight;
        self.edges.push(LexiconEdge { to, weight });
        self.edges.sort_by(|a, b| b.weight.cmp(&a.weight));
    }
}

pub struct Lexicon {
    nodes: HashMap<Particle, LexiconNode>,
}

impl Lexicon {
    /// Build a Lexicon graph from a text file and write it out
    pub fn create(filename: &str, out_filename: &str) -> (Self, usize) {
        let text = fs::read_to_string(filename)
            .unwrap_or_else(|e| panic!("Failed to read {}: {}", filename, e));

        let chars: Vec<char> = text
            .chars()
            // Strip comments (everything from # to end of line)
            .collect::<String>()
            .lines()
            .map(|line| line.split('#').next().unwrap_or(""))
            .collect::<Vec<&str>>()
            .join("\n")
            .chars()
            // Normalize word boundaries to _
            .map(|c| match c {
                '\n' | ':' | ';' | ' ' | '\t' | ',' | '.' | '!' | '?' => '_',
                c => c,
            })
            // Collapse consecutive underscores to one
            .collect::<String>()
            .split('_')
            .filter(|s| !s.is_empty())
            .collect::<Vec<&str>>()
            .join("_")
            .chars()
            .collect();

        let mut nodes: HashMap<Particle, LexiconNode> = HashMap::new();
        let mut new_node_count = 0;

        if chars.len() < PARTICLE_SIZE {
            panic!("File too short");
        }

        let mut prev_particle: Option<Particle> = None;

        for i in 0..=(chars.len() - PARTICLE_SIZE).min(80000) {
            let particle: Particle = chars[i..i + PARTICLE_SIZE].try_into().unwrap();

            if let std::collections::hash_map::Entry::Vacant(e) = nodes.entry(particle) {
                e.insert(LexiconNode::new(particle));
                new_node_count += 1;
            }

            if let Some(prev) = prev_particle {
                nodes.get_mut(&prev).unwrap().add_edge(particle);
            }

            prev_particle = Some(particle);
        }

        let lex = Self { nodes };
        let output = lex.print_graph();
        fs::write(out_filename, output)
            .unwrap_or_else(|e| panic!("Failed to write {}: {}", out_filename, e));

        (lex, new_node_count)
    }

    /// Read a Lexicon from a serialized file
    pub fn read(filename: &str) -> Self {
        let text = fs::read_to_string(filename)
            .unwrap_or_else(|e| panic!("Failed to read {}: {}", filename, e));

        let mut nodes: HashMap<Particle, LexiconNode> = HashMap::new();

        for line in text.lines() {
            let mut parts = line.split(':');
            let node_text = parts.next().expect("missing node text");
            let particle = str_to_particle(node_text);

            nodes
                .entry(particle)
                .or_insert_with(|| LexiconNode::new(particle));

            let edges_str = parts.next().unwrap_or("");
            let edge_parts: Vec<&str> = edges_str.split(';').collect();

            for chunk in edge_parts.chunks(2) {
                if chunk.len() < 2 {
                    break;
                }
                let to_text = chunk[0];
                let weight: u32 = chunk[1].trim().parse().unwrap_or(0);
                if weight == 0 {
                    continue;
                }

                let to_particle = str_to_particle(to_text);
                nodes
                    .entry(to_particle)
                    .or_insert_with(|| LexiconNode::new(to_particle));
                nodes
                    .get_mut(&particle)
                    .unwrap()
                    .create_edge(to_particle, weight);
            }
        }

        Self { nodes }
    }

    /// Generate a random word from the phonicon
    pub fn generate_word(&self, max_length: usize) -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        loop {
            // Pick a random node that starts with a capital letter
            let capitals: Vec<Particle> = self
                .nodes
                .keys()
                .filter(|p| p[0].is_uppercase())
                .copied()
                .collect();

            if capitals.is_empty() {
                panic!("No capital particles found");
            }

            let start = capitals[rng.gen_range(0..capitals.len())];
            let mut out = String::new();
            for c in start {
                out.push(c);
            }

            let mut current = start;

            loop {
                if out.len() > max_length + 16 {
                    break;
                }

                let node = match self.nodes.get(&current) {
                    Some(n) if !n.edges.is_empty() => n,
                    _ => break,
                };

                // Choose edge weighted by frequency
                let temperature = 1.5;
                let mut chosen = None;
                let total: f64 = node
                    .edges
                    .iter()
                    .map(|e| (e.weight as f64).powf(1.0 / temperature))
                    .sum();
                let rand_prob = rng.gen_range(0.0..total);
                let mut accum = 0.0f64;
                for edge in &node.edges {
                    accum += (edge.weight as f64).powf(1.0 / temperature);
                    if accum > rand_prob {
                        chosen = Some(edge.to);
                        break;
                    }
                }

                let next = match chosen {
                    Some(p) => p,
                    None => break,
                };

                // End of word marker
                if next[PARTICLE_SIZE - 1] == '_' {
                    break;
                }

                out.push(next[PARTICLE_SIZE - 1]);
                current = next;
            }

            if (4..=max_length).contains(&out.len()) {
                return out;
            }
        }
    }

    /// Serialize the graph to a string
    pub fn print_graph(&self) -> String {
        let mut out = String::new();
        for node in self.nodes.values() {
            let text: String = node.text.iter().collect();
            out.push_str(&text);
            out.push(':');
            for edge in &node.edges {
                let to: String = edge.to.iter().collect();
                out.push_str(&format!("{};{};", to, edge.weight));
            }
            out.push('\n');
        }
        out
    }
}

fn str_to_particle(s: &str) -> Particle {
    let chars: Vec<char> = s.chars().collect();
    assert!(chars.len() >= PARTICLE_SIZE, "particle too short: {:?}", s);
    chars[0..PARTICLE_SIZE].try_into().unwrap()
}
