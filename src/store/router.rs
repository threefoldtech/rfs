use std::ops::RangeInclusive;

/// route implements a naive prefix router by going through the complete set of
/// available routers and find that ones that matches this given prefix
#[derive(Default, Clone)]
pub struct Router<T> {
    pub(crate) routes: Vec<(RangeInclusive<u8>, T)>,
}

impl<T> Router<T> {
    pub fn new() -> Self {
        Self {
            routes: Vec::default(),
        }
    }

    /// add a range
    pub fn add(&mut self, start: u8, end: u8, route: T) {
        self.routes.push((start..=end, route));
    }

    /// return all stores that matches a certain key
    ///
    /// TODO: may be they need to be randomized
    pub fn route(&self, i: u8) -> impl Iterator<Item = &T> {
        self.routes
            .iter()
            .filter(move |f| f.0.contains(&i))
            .map(|v| &v.1)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test() {
        let mut router = Router::default();

        router.add(0, 255, "a");
        router.add(0, 255, "b");
        router.add(0, 128, "c");

        let paths: Vec<&str> = router.route(200).map(|v| *v).collect();
        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0], "a");
        assert_eq!(paths[1], "b");

        let paths: Vec<&str> = router.route(0).map(|v| *v).collect();
        assert_eq!(paths.len(), 3);
        assert_eq!(paths[0], "a");
        assert_eq!(paths[1], "b");
        assert_eq!(paths[2], "c");
    }
}
