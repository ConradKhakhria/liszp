use std::rc::Rc;

pub struct Error {
    filename: Option<Rc<String>>,
    message: Rc<String>,
    stack_trace: Vec<Rc<String>>
}


impl Error {

    /* Instantiation */

    pub fn new<S: ToString>(message: S) -> Self {
        /* Creates a new error */

        Self {
            filename: None,
            message: Rc::new(message.to_string()),
            stack_trace: vec![]
        }
    }


    /* Transformation */


    pub fn add_filename<S: ToString>(&self, filename: S) -> Self {
        /* Creates a copy of self with a filename */

        let filename = Some(Rc::new(filename.to_string()));
        let message = Rc::clone(&self.message);
        let mut stack_trace = Vec::with_capacity(self.stack_trace.len());

        for line in self.stack_trace.iter() {
            stack_trace.push(Rc::clone(line));
        }

        Self {
            filename,
            message: Rc::clone(&self.message),
            stack_trace
        }
    }


    pub fn add_stack_trace_step<S: ToString>(&self, function_name: Option<S>) -> Self {
        /* Adds a new element to the stack trace */

        let mut stack_trace = Vec::with_capacity(self.stack_trace.len());

        for line in self.stack_trace.iter() {
            stack_trace.push(Rc::clone(line));
        }

        stack_trace.push(Rc::new(
            match function_name {
                Some(fname) => format!("-> in function '{}'", fname.to_string()),
                None => "-> in lambda function".into()
            }
        ));

        Self {
            filename: match &self.filename {
                Some(v) => Some(Rc::clone(v)),
                None => None
            },
            message: Rc::clone(&self.message),
            stack_trace
        }
    }


    /* Display */


    pub fn display(&self, full_trace: bool) -> String {
        /* Creates a string repr of an error message */

        let trace_display_count = if full_trace {
            self.stack_trace.len() - 1
        } else {
            4
        };

        let mut message = match &self.filename {
            Some(fname) => format!("Liszp: error in '{}'", fname),
            None => "Liszp: error in <repl>".into()
        };

        message = format!("{}\n{}\nstack trace:", message, &self.message);

        for scope in self.stack_trace.iter().rev().take(trace_display_count) {
            message = format!("{}\n{}", message, scope);
        }

        message
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
