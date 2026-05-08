mod media;
mod multivariant;
mod playlist;
mod segment;
mod shared;
mod types;

/**
 * 
 * WORKFLOW:::::
 * say, we want to parse a multivariant playlist
 * 1. reaad the file or sumn
 * 2. ensure it has #EXTM3U as first line, discard if it doesn't
 * 3. loop through the lines:
 * 3.1 if line starts with #, parse it as a tag
 * 3.1.1 
 * 3.2 if line doesn't start with #, parse it as a media entry
 * 3.3 if line is empty, skip it
 * 4. 
 */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        // let result = add(2, 2);
        // assert_eq!(result, 4);
    }
}
