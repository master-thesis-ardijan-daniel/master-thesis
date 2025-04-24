use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Coordinate {
    pub lat: f32,
    pub lon: f32,
}

#[derive(Deserialize)]
pub struct TileRef<T> {
    pub tile: Vec<Vec<T>>,
    pub bounds: Bounds,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Bounds {
    pub north_west: Coordinate,
    pub south_east: Coordinate,
}

impl Bounds {
    pub fn contains(&self, coordinate: &Coordinate) -> bool {
        let lat = self.north_west.lat >= coordinate.lat && coordinate.lat >= self.south_east.lat;
        let lon = self.north_west.lon <= coordinate.lon && coordinate.lon <= self.south_east.lon;

        return lat && lon;
    }

    pub fn intersects(&self, area: &Bounds) -> bool {
        if self.north_west.lon > area.south_east.lon || area.south_east.lat > self.north_west.lat {
            return false;
        }

        if self.south_east.lon < area.north_west.lon || area.north_west.lat < self.south_east.lat {
            return false;
        }

        true
    }
}

mod tests {
    use crate::{Bounds, Coordinate};

    #[test]
    fn contains() {
        let bounds = Bounds {
            north_west: Coordinate { lon: -1., lat: 1. },
            south_east: Coordinate { lon: 1., lat: -1. },
        };

        assert!(!bounds.contains(&Coordinate { lat: -2., lon: -2. }));
        assert!(!bounds.contains(&Coordinate { lat: 2., lon: -2. }));
        assert!(!bounds.contains(&Coordinate { lat: -2., lon: 2. }));
        assert!(!bounds.contains(&Coordinate { lat: 2., lon: 2. }));

        assert!(bounds.contains(&Coordinate { lat: 0., lon: 0. }));
    }

    #[test]
    fn intersects() {
        let bounds = Bounds {
            north_west: Coordinate { lon: -1., lat: 1. },
            south_east: Coordinate { lon: 1., lat: -1. },
        };

        assert!(bounds.intersects(&Bounds {
            north_west: Coordinate { lat: 0., lon: 0. },
            south_east: Coordinate { lat: 0., lon: 0. },
        }));

        assert!(!bounds.intersects(&Bounds {
            north_west: Coordinate { lat: 3., lon: 3. },
            south_east: Coordinate { lat: 4., lon: 4. },
        }));

        assert!(!bounds.intersects(&Bounds {
            north_west: Coordinate { lat: 3., lon: -3. },
            south_east: Coordinate { lat: 4., lon: -4. },
        }));

        assert!(!bounds.intersects(&Bounds {
            north_west: Coordinate { lat: -3., lon: -3. },
            south_east: Coordinate { lat: -4., lon: -4. },
        }));

        assert!(!bounds.intersects(&Bounds {
            north_west: Coordinate { lat: -3., lon: 3. },
            south_east: Coordinate { lat: -4., lon: 4. },
        }));

        assert!(bounds.intersects(&Bounds {
            north_west: Coordinate { lat: 0., lon: 0. },
            south_east: Coordinate { lat: -4., lon: 4. },
        }));

        assert!(bounds.intersects(&Bounds {
            north_west: Coordinate { lat: 0., lon: -4. },
            south_east: Coordinate { lat: -4., lon: 0. },
        }));

        assert!(bounds.intersects(&Bounds {
            north_west: Coordinate { lat: 4., lon: -4. },
            south_east: Coordinate { lat: 0., lon: 0. },
        }));

        assert!(bounds.intersects(&Bounds {
            north_west: Coordinate { lat: 4., lon: 0. },
            south_east: Coordinate { lat: 0., lon: 4. },
        }));

        assert!(bounds.intersects(&Bounds {
            north_west: Coordinate { lat: -1., lon: -1. },
            south_east: Coordinate {
                lat: 0.5,
                lon: -0.5,
            },
        }));
    }
}
