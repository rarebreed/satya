//! # `satya-maps`
//!
//! Continuous metric (`f32` m / `i32` cm) tactical battlemap engine, analytical LoS/cover,
//! gait reachability, barrier ballistic penetration vs. concealment, seamless edge-matched
//! procedural continuation, and dressed procedural generators across all 25 sector presets.

pub mod barrier_ballistics;
pub mod map_continuation;
pub mod map_continuation_ribbons;
pub mod map_dressing;
pub mod map_estate_grammars;
pub mod map_gen_agro_industrial;
pub mod map_gen_awakened;
pub mod map_gen_civic_retail;
pub mod map_gen_commercial;
pub mod map_gen_residential;
pub mod map_gen_zoning;
pub mod map_home_floorplan;
pub mod map_interior_gen;
pub mod map_interior_topologies;
pub mod map_narrative_manifest;
pub mod map_outdoor_gen;
pub mod map_outdoor_natural;
pub mod map_pool_spa_gen;
pub mod map_room_furnish;
pub mod map_sector_gen;
pub mod map_sector_scifi;
pub mod map_surface_geom;
pub mod map_surface_queries;
pub mod map_urban_grammars;
pub mod map_world_graph;
pub mod map_world_transitions;
pub mod tactical_map;
pub mod tactical_map_edges;
pub mod tactical_map_geom;
pub mod tactical_map_inspect;
pub mod tactical_map_reach;
pub mod tactical_map_spatial;

pub use barrier_ballistics::*;
pub use map_dressing::*;
pub use map_narrative_manifest::*;
pub use map_surface_geom::*;
pub use tactical_map::*;
pub use tactical_map_geom::*;

#[cfg(test)]
mod tests_map_world;
#[cfg(test)]
mod tests_tactical_map;
