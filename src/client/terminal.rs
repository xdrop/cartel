#[macro_export]
macro_rules! tprint {
    ( $( $x:expr ),* ) => {
            println!($(
                $x,
            )*);
    };
}

#[macro_export]
macro_rules! teprint {
    ( $x:expr ) => {
        println!("{} {}", console::style("Error:").bold().red(), $x);
    };
}

#[macro_export]
macro_rules! texit {
    ($x:expr) => {{
        teprint!($x);
        std::process::exit(1);
    }};
}

macro_rules! tprintstep {
    ($message:expr,$step:expr,$of:expr,$emoji:expr) => {
        tprint!(
            "{} {} {}",
            console::style(concat!(
                "[",
                stringify!($step),
                "/",
                stringify!($of),
                "]"
            ))
            .bold()
            .dim(),
            $emoji,
            $message
        )
    };
}
