use std::fmt;
use std::result;

use pest;

#[derive(Parser)]
#[grammar = "grammar.pest"]
pub struct RuleParser;

pub type RegulationPair<'a> = pest::iterators::Pair<Rule, pest::inputs::StrInput<'a>>;
pub type RegulationPairs<'a> = pest::iterators::Pairs<Rule, pest::inputs::StrInput<'a>>;

#[derive(Clone)]
#[derive(Copy)]
#[derive(Debug)]
#[derive(PartialEq)]
#[repr(C)]
pub enum Day {
    Sunday,
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
}

#[derive(Clone)]
#[derive(Copy)]
#[derive(Debug)]
#[derive(PartialEq)]
#[repr(C)]
pub enum Month {
    January,
    February,
    March,
    April,
    May,
    June,
    July,
    August,
    September,
    October,
    November,
    December,
}

#[derive(Clone)]
#[derive(Copy)]
#[derive(Debug)]
#[derive(PartialEq)]
#[repr(C)]
pub struct Time {
    pub hours: u8,
    pub minutes: u8,
}

#[derive(Clone)]
#[derive(Copy)]
#[derive(Debug)]
#[derive(Eq)]
#[derive(PartialEq)]
pub enum Span<Type>
where
    Type: Clone,
{
    Range(Type, Type),
    All,
}

pub trait Rangeable {
    fn invert(&mut self);
}

impl<Type> Rangeable for Span<Type>
where
    Type: Clone,
{
    fn invert(&mut self) {
        *self = match *self {
            Span::Range(ref start, ref end) => Span::Range(end.clone(), start.clone()),
            Span::All => Span::All,
        };
    }
}

#[derive(Clone)]
#[derive(Debug)]
#[derive(PartialEq)]
pub struct Date {
    pub weekdays: Span<Day>,
    pub hours: Span<Time>,
    pub months: Span<(u8, Month)>,
}

#[derive(Clone)]
#[derive(Debug)]
#[derive(PartialEq)]
#[repr(C)]
pub enum Vehicle {
    Private,
    Commercial,
    Trucks,
}

#[derive(Clone)]
#[derive(Debug)]
pub enum TimeLimit {
    Minutes(u16),
    Infinite,
}

#[derive(Clone)]
#[derive(Debug)]
#[repr(C)]
pub enum Action {
    Parking,
    Standing,
    Stopping,
}

pub struct ParkingRule {
    pub active_dates: Vec<Date>,
    pub action: Action,
    pub is_inverted: bool,
    pub time_limit: TimeLimit,
    pub exclusive_vehicle_types: Vec<Vehicle>,
}

impl Day {
    pub fn from_pair(pair: RegulationPair) -> Day {
        match pair.as_rule() {
            Rule::sunday => Day::Sunday,
            Rule::monday => Day::Monday,
            Rule::tuesday => Day::Tuesday,
            Rule::wednesday => Day::Wednesday,
            Rule::thursday => Day::Thursday,
            Rule::friday => Day::Friday,
            Rule::saturday => Day::Saturday,
            _ => unreachable!("Day::from_pair"),
        }
    }

    pub fn span_except(day: Day) -> Span<Day> {
        match day {
            Day::Sunday => Span::Range(Day::Monday, Day::Saturday),
            Day::Monday => Span::Range(Day::Tuesday, Day::Sunday),
            Day::Tuesday => Span::Range(Day::Wednesday, Day::Monday),
            Day::Wednesday => Span::Range(Day::Thursday, Day::Tuesday),
            Day::Thursday => Span::Range(Day::Friday, Day::Wednesday),
            Day::Friday => Span::Range(Day::Saturday, Day::Thursday),
            Day::Saturday => Span::Range(Day::Sunday, Day::Friday),
        }
    }
}

impl Month {
    pub fn from_pair(pair: RegulationPair) -> Month {
        match pair.as_rule() {
            Rule::january => Month::January,
            Rule::february => Month::February,
            Rule::march => Month::March,
            Rule::april => Month::April,
            Rule::may => Month::May,
            Rule::june => Month::June,
            Rule::july => Month::July,
            Rule::august => Month::August,
            Rule::september => Month::September,
            Rule::october => Month::October,
            Rule::november => Month::November,
            Rule::december => Month::December,
            _ => unreachable!("Month::from_pair"),
        }
    }
}

impl fmt::Debug for ParkingRule {
    fn fmt(&self, f: &mut fmt::Formatter) -> result::Result<(), fmt::Error> {
        write!(
            f,
            "ParkingRule {{\n  active_dates: {:?}\n  action: {:?}\n  is_inverted: {:?}\n  time_limit: {:?}\n  exclusive_vehicle_types: {:?}\n}}",
            self.active_dates,
            self.action,
            self.is_inverted,
            self.time_limit,
            self.exclusive_vehicle_types
        )
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_range() {
        use parking::Rangeable;

        let mut r = parking::Span::Range(5, 10);
        r.invert();
        assert_eq!(r, parking::Span::Range(10, 5));
    }
}
