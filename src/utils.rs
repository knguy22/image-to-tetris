pub fn split_into_n_chunks<T, I>(iter: I, n: usize) -> Vec<Vec<T>> 
    where I: Iterator<Item = T>
{
    let mut chunks = Vec::new();
    for _ in 0..n {
        chunks.push(Vec::new());
    }

    for (i, item) in iter.enumerate() {
        chunks[i % n].push(item);
    }
    chunks
}
