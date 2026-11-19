//! Typed ids. Content names are interned to these at load time.

use serde::{Deserialize, Serialize};

macro_rules! id {
    ($name:ident, $t:ty) => {
        #[derive(
            Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
        )]
        pub struct $name(pub $t);

        impl $name {
            pub fn idx(self) -> usize {
                self.0 as usize
            }
        }
    };
}

id!(EntityId, u32);
id!(TeamId, u16);
id!(StatId, u16);
id!(ResourceId, u16);
id!(FlagId, u16);
id!(TagId, u16);
id!(EventKind, u16);
id!(CustomId, u16);
id!(UnitId, u16);
id!(ActionId, u16);
id!(BuffId, u16);
id!(PerkId, u16);
id!(EffectListId, u32);
