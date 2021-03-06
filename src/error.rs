pub struct Error(String);


impl Error {
    pub fn new<T: Into<String>>(message: T) -> Error {
        /* Creates a new error */

        Error(message.into())
    }


    pub fn println(&self) {
        /* Displays an error */

        eprintln!("Liszp: {}", self.0);
    }
}


impl<T> Into<Result<T, Error>> for Error {
    fn into(self) -> Result<T, Error> {
        /* Turns Error into a result */

        Err(self)
    }
}


#[macro_export]
macro_rules! new_error {
    ($msg:literal) => {
        Error::new($msg)
    };

    ($msg:literal, $($format_parameter:expr),*) => {
        Error::new(format!($msg, $($format_parameter),*))
    };
}
