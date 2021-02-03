#[macro_use]
extern crate polars;
use anyhow::Result;
use chrono::prelude::*;
use chrono::{Date, DateTime, Duration, Local, Utc};
use ndarray::prelude::*;
use polars::prelude::*;

#[test]
fn example() -> Result<()> {
    let weekday = Utc.ymd(2020, 4, 1).weekday();
    print!("{}", weekday);
    Ok(())
}

fn print_typename<T>(_: T) {
    println!("{}", std::any::type_name::<T>());
}

fn fiscal_year_first_date(date: Date<Utc>) -> Date<Utc> {
    let mut y = date.year();
    let m = date.month();
    if (m == 1 || m == 2 || m == 3) {
        y = y - 1;
    }
    return Utc.ymd(y, 4, 1);
}

fn main() {
    let events = vec![
        Utc.ymd(2014, 1, 14),
        Utc.ymd(2015, 1, 13),
        Utc.ymd(2016, 1, 12),
    ];
    let first = fiscal_year_first_date(events[0]);
    let last = events.last();
    let range_candidates: Vec<i64> = (-3..4).collect();
    let range_recurrence = vec![events[0], events[2]];
    let forecasted = forecast(range_recurrence, range_candidates, events);
}

fn weekdays(date: &Date<Utc>) -> Weekday {
    date.weekday()
}

fn weekdays_considering_nholiday(dates: &Vec<Date<Utc>>) -> Vec<String> {
    dates
        .iter()
        .map(|date| weekdays(&date).to_string())
        .collect()
}

fn monthweek(date: &Date<Utc>) -> u32 {
    (date.day() - 1) / 7 + 1
}

fn monthweeks(dates: &Vec<Date<Utc>>) -> Vec<u32> {
    dates.iter().map(|date| monthweek(&date)).collect()
}

fn months(dates: &Vec<Date<Utc>>) -> Vec<u32> {
    dates.iter().map(|date| date.month()).collect()
}

fn get_params_list(dates: &Vec<Date<Utc>>) -> DataFrame {
    let wdays = weekdays_considering_nholiday(dates);
    let weeks = monthweeks(dates);
    let months = months(dates);
    // let holidays = holidays(&dates);
    //
    let plist = df!("wday" => &wdays,
		    "weeks" => &weeks,
		    "months" => &months)
    .unwrap();

    println!("{}", plist);

    plist
}

fn dates_to_occurreds(dates: &Vec<Date<Utc>>, range: &Vec<Date<Utc>>) -> Vec<u64> {
    let len = (range[1] - range[0]).num_days() + 1;
    let mut occurreds = vec![0; len as usize];

    let seq_dates: Vec<Date<Utc>> = (0..len).map(|x| range[0] + Duration::days(x)).collect();

    for date in dates.iter() {
        for (j, seq_date) in seq_dates.iter().enumerate() {
            if date == seq_date {
                occurreds[j] = 1;
            }
        }
    }

    occurreds
}

fn get_ac(f: &Vec<u64>, range: &Vec<Date<Utc>>) -> Vec<f64> {
    let start = 0;
    let end = (range[1] - range[0]).num_days();

    let mut ac: Vec<f64> = vec![0.0; (end + 1) as usize];

    for lag in 0..=end {
        let mut p = Vec::new();
        for i in 0..(f.len() as i64 - lag) {
            p.push(f[i as usize] * f[lag as usize]);
        }
        ac[lag as usize] = p.iter().sum::<u64>() as f64 / p.len() as f64;
    }

    ac
}

fn get_big_wave_cycle(dates: &Vec<Date<Utc>>, range: &Vec<Date<Utc>>) -> usize {
    let series = dates_to_occurreds(dates, range);
    let mut ac = get_ac(&series, range);

    println!("{:?}", ac);

    // 要修正
    // 長過ぎる周期をカット
    if ac.len() > 400 {
        ac = ac[..400].to_vec();
    }

    // 最大値の index = 周期となるためこうしてるけどもっといい方法がありそう
    let mut max = 0.0;
    let mut max_index = 0;
    for (i, &x) in ac.iter().enumerate() {
        if x > max {
            max = x;
            max_index = i;
        }
    }

    if ac.len() == 0 {
        0
    } else {
        max_index
    }
}

fn closest_event_index(events: &Vec<Date<Utc>>, date: Date<Utc>) -> usize {
    let last = events.len() - 1;
    for i in 0..=last {
        if events[i] <= date && date <= events[i + 1] {
            if date - events[i] < events[i + 1] - date {
                i
            } else {
                i + 1
            };
        }
    }

    if events[last] < date {
        last
    } else {
        0
    }
}

fn get_candidates(events: &Vec<Date<Utc>>, range: &Vec<i64>, period: usize) -> Vec<Date<Utc>> {
    let latest = events.last().unwrap();
    let criterion = *latest - Duration::days(period as i64);

    let i = closest_event_index(events, criterion);
    let mut d = (events[i + 1] - events[i]).num_days();
    if d > 365 {
        d = 365;
    }
    let pivot = *latest + Duration::days(d);
    let candidates: Vec<Date<Utc>> = range.iter().map(|x| pivot + Duration::days(*x)).collect();

    candidates
}

fn gen_lm(cdv: &Series) -> Array2<u64> {
    let nrow = cdv.len();
    let col_uniq = cdv.unique().unwrap();
    let ncol = col_uniq.len();
    let mut m: Array2<u64> = Array::zeros((nrow, ncol));

    for row in cdv.utf8().into_iter() {
        for col in col_uniq.utf8().into_iter() {
            println!("{:?}", row);
            println!("{:?}", col);
            // if row.lt(col).all_true() {
            //     m[[i, j]] = 1;
            // }
        }
    }

    println!("{:?}", m);
    m
}

fn get_lm_all(plist: DataFrame, first: Date<Utc>, last: Date<Utc>) {
    let len = (last - first).num_days();
    let dates: Vec<Date<Utc>> = (0..=len).map(|x| first + Duration::days(x)).collect();
    let mut plist_alldate = get_params_list(&dates);

    let lm = gen_lm(plist_alldate.column("wday").unwrap());
}

fn forecast(range_recurrence: Vec<Date<Utc>>, range_candidate: Vec<i64>, events: Vec<Date<Utc>>) {
    let first = range_recurrence[0];
    let last = range_recurrence[1];
    let recurrence = events;
    let recurrence_plist = get_params_list(&recurrence);

    let mut period = get_big_wave_cycle(&recurrence, &range_recurrence);
    if period == 0 {
        period = 365
    }

    let candidates = get_candidates(&recurrence, &range_candidate, period);
    let candidates_plist = get_params_list(&candidates);

    let lm = get_lm_all(recurrence_plist, first, last);
}
