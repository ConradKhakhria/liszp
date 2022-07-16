pub struct Error(String);


impl Error {
    pub fn new<T: Into<String>>(message: T) -> Error {
        /* Creates a new error */

        Error(message.into())
    }


    pub fn print(self) -> Self {
        /* Displays an error and then returns it immediately */

        eprintln!("Liszp: {}", self.0);

        self
    }
}
