//! `ENPCHoliday` (`Bundles/NPCHolidayCondition.cs`). The active holiday is `Provider.authorityHoliday`, which the
//! server sends, so it can hold any value; the holiday schedule comes later (V14).

pinewatch_mono::net_enum! {
    /// `ENPCHoliday`.
    pub struct ENPCHoliday(i32) {
        NONE = 0,
        HALLOWEEN = 1,
        CHRISTMAS = 2,
        APRIL_FOOLS = 3,
        VALENTINES = 4,
        PRIDE_MONTH = 5,
        LUNAR_NEW_YEAR = 6,
        /// July 7th.
        UNTURNED_ANNIVERSARY = 7,
        /// Used to determine the number of holidays.
        MAX = 8,
    }
}
