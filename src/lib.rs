use std::ffi::CStr;

extern crate pest;
#[macro_use]
extern crate pest_derive;

use pest::Parser;

pub mod parking;
use self::parking::*;

fn process_days(pair: RegulationPair) -> Vec<Span<Day>> {
    pair.into_inner()
        .map(|inner| match inner.as_rule() {
            Rule::day => {
                let day = Day::from_pair(inner.into_inner().next().unwrap());
                Span::Range(day, day)
            }
            Rule::school => Span::Range(Day::Monday, Day::Friday),
            Rule::all => Span::All,
            _ => unreachable!(),
        })
        .collect()
}

fn process_months(pair: RegulationPair) -> Vec<Month> {
    pair.into_inner()
        .map(|inner| match inner.as_rule() {
            Rule::month => Month::from_pair(inner.into_inner().next().unwrap()),
            _ => unreachable!(),
        })
        .collect()
}

fn process_time(pair: RegulationPair) -> Time {
    let mut hours = 0;
    let mut minutes = 0;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::hours => hours = inner.into_span().as_str().trim().parse::<u8>().unwrap(),
            Rule::minutes => minutes = inner.into_span().as_str().trim().parse::<u8>().unwrap(),
            Rule::meridiem => {
                if inner.into_span().as_str().to_lowercase() == "pm" {
                    hours += 12; // always happens after Rule::hours
                }
            }
            Rule::midday => {
                hours = 12;
                minutes = 0;
            }
            Rule::midnight => {
                hours = 0;
                minutes = 0;
            }
            _ => unreachable!(),
        }
    }
    Time { hours, minutes }
}

fn process_day_range(pair: RegulationPair) -> Vec<Span<Day>> {
    let mut iterator = pair.into_inner();
    let first = iterator.next().unwrap();
    match first.as_rule() {
        Rule::day => {
            let start_day = Day::from_pair(first.into_inner().next().unwrap());
            let end_day = Day::from_pair(iterator.next().unwrap().into_inner().next().unwrap());

            vec![Span::Range(start_day, end_day)]
        }
        Rule::days => process_days(first),
        _ => unreachable!(),
    }
}

fn process_month_range(pair: RegulationPair) -> Vec<Span<(u8, Month)>> {
    let mut iterator = pair.into_inner();
    let first = iterator.next().unwrap();
    match first.as_rule() {
        Rule::month => {
            let start_month = Month::from_pair(first.into_inner().next().unwrap());
            let end_month = Month::from_pair(iterator.next().unwrap().into_inner().next().unwrap());

            vec![Span::Range((1, start_month), (31, end_month))]
        }
        Rule::months => {
            process_months(first)
                .iter()
                .map(|m| Span::Range((1, *m), (31, *m)))
                .collect()
        }
        _ => unreachable!(),
    }
}

fn process_time_range(pair: RegulationPair) -> Span<Time> {
    let mut iterator = pair.into_inner();
    let first = iterator.next().unwrap();
    match first.as_rule() {
        Rule::time => {
            let start_time = process_time(first);
            let end_time = process_time(iterator.next().unwrap());

            Span::Range(start_time, end_time)
        }
        Rule::digit => {
            let mut number_string = first.as_str().to_owned();
            let mut next_pair = iterator.next();
            while next_pair.is_some() && next_pair.clone().unwrap().as_rule() == Rule::digit {
                number_string.push_str(next_pair.clone().unwrap().as_str());
                next_pair = iterator.next();
            }

            let start_time = Time {
                hours: number_string.trim().parse::<u8>().unwrap(),
                minutes: 0,
            };
            let end_time = process_time(next_pair.unwrap());

            Span::Range(start_time, end_time)
        }
        Rule::anytime => Span::All,
        _ => unreachable!(),
    }
}

fn process_range(pair: RegulationPair) -> Vec<Date> {
    let mut dates: Vec<Date> = Vec::new();
    let mut day_ranges = Vec::new();
    let mut month_ranges = Vec::new();
    let mut time_ranges = Vec::new();

    for range in pair.into_inner() {
        match range.as_rule() {
            Rule::day_range => {
                day_ranges.append(&mut process_day_range(range));
            }
            Rule::month_range => {
                month_ranges.append(&mut process_month_range(range));
            }
            Rule::time_range => {
                time_ranges.push(process_time_range(range));
            }
            Rule::exception_day => {
                let day = Day::from_pair(
                    range
                        .into_inner()
                        .next()
                        .unwrap()
                        .into_inner()
                        .next()
                        .unwrap(),
                );

                if day_ranges.is_empty() {
                    day_ranges.push(Day::span_except(day));
                } else {
                    for mut day_range in day_ranges.iter_mut() {
                        match *day_range {
                            Span::Range(_, _) => continue, // generally does not
                            Span::All => {
                                *day_range = Day::span_except(day);
                            }
                        }
                    }
                }
            }
            _ => unreachable!(),
        }
    }

    if day_ranges.is_empty() {
        day_ranges.push(Span::All);
    }

    if month_ranges.is_empty() {
        month_ranges.push(Span::All);
    }

    if time_ranges.is_empty() {
        time_ranges.push(Span::All);
    }

    for day_range in day_ranges.iter() {
        for month_range in month_ranges.iter() {
            for time_range in time_ranges.iter() {
                dates.push(Date {
                    weekdays: *day_range,
                    months: *month_range,
                    hours: *time_range,
                })
            }
        }
    }

    dates
}

fn process_duration(pair: RegulationPair) -> TimeLimit {
    fn digits_to_u16(iterator: &mut std::iter::Peekable<RegulationPairs>) -> u16 {
        iterator
            .take_while(|pair| pair.as_rule() == Rule::digit)
            .fold(String::new(), |acc, pair| acc.to_owned() + pair.as_str())
            .parse::<u16>()
            .unwrap()
    };

    let mut iter = pair.into_inner().peekable();
    let numerator = digits_to_u16(&mut iter);
    let denominator = if iter.peek().is_some() {
        digits_to_u16(&mut iter)
    } else {
        1
    };

    TimeLimit::Minutes((numerator * 60 / denominator))
}

fn process_vehicle_modifier(pair: RegulationPair) -> Vec<Vehicle> {
    let inner = pair.into_inner().next().unwrap();
    let vehicle = match inner.clone().into_inner().next().unwrap().as_rule() {
        Rule::trucks => Some(Vehicle::Trucks),
        Rule::commercial => Some(Vehicle::Commercial),
        Rule::horse_cabs => None,
        _ => unreachable!(),
    };

    if let Some(v) = vehicle {
        match inner.as_rule() {
            Rule::vehicle_only => vec![v],
            Rule::vehicle_exception => {
                let mut vehicles = Vec::new();

                if v != Vehicle::Private {
                    vehicles.push(Vehicle::Private);
                }

                if v != Vehicle::Commercial {
                    vehicles.push(Vehicle::Commercial);
                }

                if v != Vehicle::Trucks {
                    vehicles.push(Vehicle::Trucks);
                }

                vehicles
            }
            _ => unreachable!(),
        }
    } else {
        vec![]
    }
}

fn process_regulation(pair: RegulationPair) -> Vec<ParkingRule> {
    let mut action = Action::Parking;
    let mut allowed_vehicles = Vec::new();
    let mut is_inverted = false;
    let mut limit = TimeLimit::Infinite;
    let mut rules = Vec::new();

    for inner in pair.clone().into_inner() {
        match inner.as_rule() {
            Rule::negative => is_inverted = true,
            Rule::duration => limit = process_duration(inner),
            Rule::metered => continue,
            Rule::parking => action = Action::Parking,
            Rule::standing => action = Action::Standing,
            Rule::stopping => action = Action::Stopping,
            Rule::vehicle_modifier => allowed_vehicles = process_vehicle_modifier(inner),
            Rule::range => {
                rules.push(ParkingRule {
                    action: action.clone(),
                    is_inverted: is_inverted,
                    active_dates: process_range(inner),
                    time_limit: limit.clone(),
                    exclusive_vehicle_types: allowed_vehicles.clone(),
                });
            }
            _ => unreachable!(),
        }
    }

    if rules.is_empty() {
        rules.push(ParkingRule {
            action: action.clone(),
            is_inverted: is_inverted,
            active_dates: vec![
                Date {
                    weekdays: Span::All,
                    hours: Span::All,
                    months: Span::All,
                },
            ],
            time_limit: limit.clone(),
            exclusive_vehicle_types: allowed_vehicles.clone(),
        });
    }

    rules
}

pub fn parse_str(input: &str) -> Vec<ParkingRule> {
    let pairs = parking::RuleParser::parse(Rule::base, input).unwrap();

    pairs
        .clone()
        .next()
        .unwrap()
        .into_inner()
        .flat_map(|pair| match pair.as_rule() {
            Rule::arrow | Rule::parens | Rule::night | Rule::comment | Rule::whitespace => vec![],
            Rule::regulation => process_regulation(pair),
            Rule::day => vec![],
            _ => unreachable!(),
        })
        .collect()
}

#[repr(C)]
pub struct Array<Type> {
    data: *mut Type,
    len: usize,
}

impl<Type> Drop for Array<Type> {
    fn drop(&mut self) {
        drop(unsafe {
            Vec::from_raw_parts(self.data, self.len, self.len)
        });
    }
}

trait IntoArray<Type> {
    fn into_array(self) -> Array<Type>;
}

impl<Type> IntoArray<Type> for Vec<Type> {
    fn into_array(mut self) -> Array<Type> {
        self.shrink_to_fit();

        let arr = Array {
            len: self.len(),
            data: self.as_mut_ptr(),
        };

        std::mem::forget(self);

        arr
    }
}

#[repr(C)]
pub struct CDate {
    pub start_day: Day,
    pub end_day: Day,
    pub start_hour: Time,
    pub end_hour: Time,
    pub start_month: Month,
    pub start_month_day: u8,
    pub end_month: Month,
    pub end_month_day: u8,
}

impl CDate {
    pub fn from_date(date: &Date) -> CDate {
        CDate {
            start_day: match date.weekdays {
                Span::Range(start, _) => start,
                Span::All => Day::Sunday,
            },
            end_day: match date.weekdays {
                Span::Range(_, end) => end,
                Span::All => Day::Saturday,
            },
            start_hour: match date.hours {
                Span::Range(start, _) => start,
                Span::All => Time {
                    hours: 0,
                    minutes: 0,
                },
            },
            end_hour: match date.hours {
                Span::Range(_, end) => end,
                Span::All => Time {
                    hours: 23,
                    minutes: 59,
                },
            },
            start_month: match date.months {
                Span::Range((_, start), _) => start,
                Span::All => Month::January,
            },
            start_month_day: match date.months {
                Span::Range((start, _), _) => start,
                Span::All => 1,
            },
            end_month: match date.months {
                Span::Range(_, (_, end)) => end,
                Span::All => Month::December,
            },
            end_month_day: match date.months {
                Span::Range(_, (end, _)) => end,
                Span::All => 31,
            },
        }
    }
}

#[repr(C)]
pub struct CTimeLimit {
    minutes: u16,
}

#[repr(C)]
pub struct CParkingRule {
    pub active_dates: Array<CDate>,
    pub action: Action,
    pub is_inverted: bool,
    pub time_limit: CTimeLimit,
    pub exclusive_vehicle_types: Array<Vehicle>,
}

#[no_mangle]
pub extern "C" fn rules_from_str(input_ptr: *const i8) -> Array<CParkingRule> {
    let input = unsafe {
        assert!(!input_ptr.is_null());

        CStr::from_ptr(input_ptr)
    }.to_str()
        .unwrap_or("");

    parse_str(input)
        .iter()
        .map(|rule| {
            CParkingRule {
                active_dates: rule.active_dates
                    .iter()
                    .map(|date| CDate::from_date(date))
                    .collect::<Vec<CDate>>()
                    .into_array(),
                action: rule.action.clone(),
                is_inverted: rule.is_inverted.clone(),
                time_limit: match rule.time_limit {
                    TimeLimit::Minutes(mins) => CTimeLimit { minutes: mins },
                    TimeLimit::Infinite => CTimeLimit { minutes: 0 },
                },
                exclusive_vehicle_types: rule.exclusive_vehicle_types.clone().into_array(),
            }
        })
        .collect::<Vec<CParkingRule>>()
        .into_array()
}

#[no_mangle]
pub extern "C" fn free_rules(rules: Array<CParkingRule>) {
    drop(rules);
}
