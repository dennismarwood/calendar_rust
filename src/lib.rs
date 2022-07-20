use calamine::{open_workbook, DataType, Error, Reader, Xlsx}; //RangeDeserializerBuilder, Range, ToCellDeserializer};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{env, error}; //, convert::TryInto, cell}; //NaiveDateTime};

#[derive(Debug)]
pub struct Config {
    pub xls_path: String,
}

impl Config {
    pub fn new(mut args: env::Args) -> Result<Config, &'static str> {
        if args.len() < 2 {
            error!("Need excel path to be provided.");
            return Err("Need excel path to be provided.");
        }
        args.next();
        Ok(Config {
            xls_path: args.next().unwrap(),
        })
    }
}

struct Schedule {
    start_date: NaiveDate,
    end_date: NaiveDate,
    employees: Vec<Employee>,
}

impl Schedule {
    fn new(start_date: NaiveDate, end_date: NaiveDate) -> Schedule {
        Schedule {
            start_date,
            end_date,
            employees: vec![],
        }
    }

    fn duration(&self) -> i64 {
        self.end_date
            .signed_duration_since(self.start_date)
            .num_days()
            + 1
    }
}

impl std::fmt::Display for Schedule {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "start_date: {} end_date: {} duration: {}\nEmployees:\n",
            self.start_date,
            self.end_date,
            self.duration()
        )?;
        for e in &self.employees {
            write!(f, "{}", e)?;
        }
        write!(f, "\n")
    }
}

#[derive(Debug)]
struct Employee {
    name: String,
    location: (usize, usize), //The location of the employee in the sheet
    days: Vec<Day>,
    index: usize,
}

impl Employee {
    fn new(name: String, location: (usize, usize)) -> Employee {
        Employee {
            name: name,
            location: location,
            days: vec![],
            index: 0,
        }
    }
}

impl Iterator for Employee {
    type Item = String;
    fn next(&mut self) -> Option<Self::Item> {
        if self.days.len() == 0 {
            None
        } else if self.index < self.days.len() {
            let date = self.days[self.index].date;
            let day_type = &self.days[self.index].day_type;
            self.index += 1;
            let json = json!({
                "date": date,
                "day_type": day_type
            });
            Some(json.to_string())
        } else {
            None
        }
    }
}

impl std::fmt::Display for Employee {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "\nName: {:10}", self.name)?;
        write!(
            f,
            "\nLocation: [{:<2}, {:<2}] ",
            self.location.0, self.location.1
        )?;
        for d in &self.days {
            write!(f, "\n{}", d)?;
        }
        write!(f, "")
    }
}

#[allow(dead_code)]
#[derive(Debug)]
struct Day {
    date: NaiveDate,
    day_type: DayType,
}

impl std::fmt::Display for Day {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} - {}", self.date, self.day_type)
    }
}

#[derive(Debug, Serialize, Deserialize)]
enum DayType {
    Off,
    Vacation,
    ADay,
    Work(NaiveDateTime),
    Undefined,
}

impl std::fmt::Display for DayType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            DayType::Off => write!(f, "Off"),
            DayType::Vacation => write!(f, "Vacation"),
            DayType::ADay => write!(f, "A Day"),
            DayType::Work(x) => write!(f, "Work - {}", x.format("%Y-%m-%d %H:%M").to_string()),
            DayType::Undefined => write!(f, "Undefined"),
        }
    }
}

pub fn process_schedule(config: Config) -> Result<(), Box<dyn error::Error>> {
    debug!("{:?}", config);

    debug!("Open and read Sheet1 of {:?}", config.xls_path);
    let mut workbook: Xlsx<_> = open_workbook(config.xls_path)?;
    let range = workbook
        .worksheet_range("Sheet1")
        .ok_or(Error::Msg("Cannot find 'Sheet1'"))??;

    let start_date = range.get_value((0, 0)).unwrap().as_date().unwrap();
    let end_date = range.get_value((1, 0)).unwrap().as_date().unwrap();
    let mut schedule = Schedule::new(start_date, end_date);
    debug!("{}", schedule);

    for row in 2..range.height() {
        debug!("{}", range.get((row, 0)).unwrap());
        let abs_location = (row, 0);
        let name = range.get((row, 0));
        match name {
            Some(name) => {
                //is there a row
                match name {
                    //is the row non empty, then treat it as a string
                    DataType::String(name) => schedule
                        .employees
                        .push(Employee::new(name.to_string(), abs_location)),
                    _ => (),
                }
            }
            None => (),
        }
    }
    debug!("{}", schedule);

    let duration: usize = schedule.duration() as usize;
    for employee in schedule.employees.iter_mut() {
        //if employee.name == "JENNY" {
        for (i, day) in schedule.start_date.iter_days().take(duration).enumerate() {
            let cell_value = range.get_value((employee.location.0 as u32, (i as u32 + 1)));
            info!("Processing - {} {}", employee.location.0 as u32, (i as u32 + 1) );
            match cell_value {
                Some(cell_value) => {
                    if let Some(x) = cell_value.get_int() {
                        employee.days.push(Day {
                            date: day,
                            day_type: DayType::Work(NaiveDateTime::new(
                                day,
                                NaiveTime::from_hms(x as u32, 0, 0),
                            )),
                        })
                    } else if let Some(x) = cell_value.get_float() {
                        employee.days.push(Day {
                            date: day,
                            day_type: DayType::Work(NaiveDateTime::new(
                                day,
                                NaiveTime::from_hms(x as u32, 0, 0),
                            )),
                        })
                    } else if let Some(x) = cell_value.get_string() {
                        match x {
                            "V" => employee.days.push(Day {
                                date: day,
                                day_type: DayType::Vacation,
                            }),
                            "A" => employee.days.push(Day {
                                date: day,
                                day_type: DayType::ADay,
                            }),
                            "X" | "M" => employee.days.push(Day {
                                date: day,
                                day_type: DayType::Off,
                            }),
                            "SC" => employee.days.push(Day {
                                date: day,
                                day_type: DayType::Work(NaiveDateTime::new(
                                    day,
                                    NaiveTime::from_hms(12, 0, 0),
                                )),
                            }),
                            "B" => employee.days.push(Day {
                                date: day,
                                day_type: DayType::Work(NaiveDateTime::new(
                                    day,
                                    NaiveTime::from_hms(12, 0, 0),
                                )),
                            }),
                            "C" => employee.days.push(Day {
                                date: day,
                                day_type: DayType::Work(NaiveDateTime::new(
                                    day,
                                    NaiveTime::from_hms(12, 0, 0),
                                )),
                            }),
                            "R" => employee.days.push(Day {
                                date: day,
                                day_type: DayType::Work(NaiveDateTime::new(
                                    day,
                                    NaiveTime::from_hms(12, 0, 0),
                                )),
                            }),
                            _ => {
                                employee.days.push(Day {
                                    date: day,
                                    day_type: DayType::Undefined,
                                });
                                error!("Undefined DayType set - {}", x);
                            }
                        }
                    } else if cell_value.is_empty() {
                        employee.days.push(Day {
                            date: day,
                            day_type: DayType::Off,
                        })
                    }
                }
                None => (),
            }
        }
        //}
    }

use std::io::{Read, Write};
use std::net::{Shutdown, TcpStream};
use std::str::from_utf8;
   /*  if let Ok(mut stream) = TcpStream::connect("127.0.1.1:5050") {
        println!("Connected to the server!");
        let test_json_string = r#"{"date":"2022-02-19","day_type":{"Work":"2022-02-19T14:00:00"}}"#;
        let x = stream.write(&test_json_string.len().to_le_bytes()[0..2]).unwrap();
        print!("{}\n", x);
        stream.write(test_json_string.as_bytes()).unwrap();
        
    } else {
        println!("Couldn't connect to server...");
    } */
     for e in schedule.employees {
        if e.name == "JENNY" {
            for d in e {
                if let Ok(mut stream) = TcpStream::connect("127.0.1.1:5050") {
                    println!("Connected to the server!");
                    stream.write(&d.len().to_le_bytes()[0..2]).unwrap();
                    stream.write(d.as_bytes()).unwrap();
                } else {
                    println!("Couldn't connect to server...");
                }
            }
        }
    }
    //info!("{}", schedUule.employees[21]);
    Ok(())
}

/* #[derive(Debug)]
struct MyTest {
    a: DataType,
    b: DataType,
} */
/* let x_res = range.range((2,0),(range.height() as u32, 0)).deserialize();
let x_rangeDeserializer = x_res?;
for x in x_rangeDeserializer { //x is a result here
    let y = match x {
        Ok(a_row) => a_row,
        Err(e) => panic!("boom")
    };
} */
/*     let mut my_iter = RangeDeserializerBuilder::new().from_range(&range.range((2,0),(range.height() as u32, 0)))?;
if let Some(result) = my_iter.next() {
    let x = result?;
    debug!("{:?}", x);
} */

/* for (row, employee) in my_iter.enumerate() {
match employee? {
    Some(employee) => {
            let (v): (String) = employee;
            debug!("row - employee: {} - {}", row, v);
            //Employee::new(t, (row + 2, 0));
    },
    None => error!("it was none")
} */

/*         let abs_location = (employee.0 + 2, employee.1);
let name = employee.2;
let name = employees.deserialize();
schedule.employees.push(Employee::new(name, abs_location)); */
//if employee.
//}

/* let r = workbook.worksheet_range("Sheet1").ok_or(Error::Msg("Cannot find 'Sheet1'"))??.range((3, 0), (4, 2));
debug!("size: {:?}", r.get_size());
debug!("{:?}", r.get((0,0))); */
//let y = r.range((3, 1), (3, 2));
//let mut my_RangeDeserializer = RangeDeserializerBuilder::new().has_headers(false).from_range(&r)?;
//let row: Vec<DataType> = my_RangeDeserializer.next().unwrap()?;
//let m: MyTest = my_RangeDeserializer.next().unwrap()?;

//for d in start_date.iter_days().take(length_of_schedule){
//debug!("{}", d);
//need to get the row for  JENNY then make a Day with the value in it (14:00) and the corresponding date above.
//}

//schedule.employees.push()

//let schedule = Schedule::new(start_date, end_date);
/* for (i, row) in range.rows().enumerate() {
       for cell in row {
           if cell.to_string() == "JENNY" {
               let start_date = range.get_value((0, 0)).unwrap();

               println!(
                   "Start Date: {} \nEnd Date: {:?} \nRow: {} \nRange Length: {} \n{:?}",
                   start_date,
                   range.get_value((1, 0)),
                   i,
                   range.width(),
                   row
               );
               let x: i64 = 3333;
               println!("{}, {:?}", x, x);
           }
       }
   }
   struct my_test {
       x: i64,
   }
*/
//let z =

/*NOTES
fn example() -> Result<(), Error> {
    //let path = format!("{}/tests/temperature.xlsx", env!("CARGO_MANIFEST_DIR"));

    //workbook is zipped excel file
    let mut workbook: Xlsx<_> = open_workbook("FEB 2022.xlsx")?;

    // range is a sheet of the excel file
    // Option<Result<Range<DataType>, XlsxError>>
    // Option
    //      Some or None
    // Result
    //      Like option but Ok or Err

    let range = workbook
        .worksheet_range("Sheet1") // returns Option<Result<Range<DataType>, XlsError>>
        //Look to see if the option is Some, if not then convert it to a Result of type Error
        //The question mark propegates the error, if there is no error it returns type of the Result
        //The ok_or removes the Option returned by .worksheet_range()
        //Two question marks here peel away the result layer of the below ok_or
        //And the result layer of .worksheet_range()
        .ok_or(Error::Msg("Cannot find 'Sheet1'"))??;

    let total_cells = range.get_size().0 * range.get_size().1;
    let non_empty_cells: usize = range.used_cells().count();
    println!(
        "Found {} cells in 'Sheet1', including {} non empty cells",
        total_cells, non_empty_cells
    );

    for (i, row) in range.rows().enumerate() {
        for cell in row {
            if cell.to_string() == "JENNY" {
                let start_date = range.get_value((0, 0)).unwrap();
                let date_time_after_a_billion_seconds = NaiveDateTime::from_timestamp(43976, 0);
                println!("{}", date_time_after_a_billion_seconds);

                println!(
                    "Start Date: {} \nEnd Date: {:?} \nRow: {} \nRange Length: {} \n{:?}",
                    start_date,
                    range.get_value((1, 0)),
                    i,
                    range.width(),
                    row
                );
                let x: i64 = 3333;
                println!("{}, {:?}", x, x);
            }
        }
    }

    for cell in range.cells() {
        if cell.2 == "JENNY" {
            println!("{:?} {}", cell, cell.2);
        }
    }

    let mut iter = RangeDeserializerBuilder::new().from_range(&range)?;

    if let Some(result) = iter.next() {
        let (label, value, v): (String, String, String) = result?;
        println!("{}-{}-{}", label, value, v);
        Ok(())
    } else {
        Err(From::from("expected at least one record but got none"))
    }
}




*/

/*


fn main() {
    #[derive(Debug)]
    struct Schedule {
        employees: Vec<Employee>,
    }


    #[derive(Debug)]
    struct Employee {
        name: String,
        work_days: Vec<WorkDays>,
    }


    #[derive(Debug)]
    struct WorkDays { //Day of work
        day_type: DayType,
    }


    #[derive(Debug)]
    enum DayType {
        Off,
        On,
    }


    let mut schedule = Schedule{employees : vec![]};

    schedule.employees.push(Employee{name: "Alpha".to_string(), work_days: vec![]});
    schedule.employees.push(Employee{name: "Betta".to_string(), work_days: vec![]});

    //This works
    for mut employee in schedule.employees.into_iter() {
        employee.work_days.push(WorkDays {day_type: DayType::Off});
    }

    //This does not.
    for mut e in schedule.employees.into_iter() {
        e.work_days.push(WorkDays {day_type: DayType::Off});
    }
}




*/
