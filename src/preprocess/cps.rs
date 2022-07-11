use crate::read::Value;
use crate::refcount_list;

use std::rc::Rc;

struct CPSConverter {
    dfs_expr_components: Vec<Rc<Value>>,
    continuation: Rc<Value>,
}


impl CPSConverter {

    /* Instantiation */

    fn new(continuation: &Rc<Value>) -> CPSConverter {
        /* Creates a new CPS converter */

        CPSConverter {
            dfs_expr_components: Vec::new(),
            continuation: continuation.clone()
        }
    }


    /* Expression rearranging */

    fn find_conditional(&self, expr: &Rc<Value>) -> Option<Rc<Value>> {
        /* Returns a Rc<> of an if expression, if expr contains one */

        match &**expr {
            Value::Cons { car, cdr } => {
                if car.name() == "&if" {
                    Some(expr.clone())
                } else if vec![ "&lambda", "&quote" ].contains(&car.name().as_str()) {
                    None
                } else if let Some(cond) = self.find_conditional(car) {
                    Some(cond)
                } else if let Some(cond) = self.find_conditional(cdr) {
                    Some(cond)
                } else {
                    None
                }
            }

            _ => None
        }
    }


    fn move_conditionals_to_top_level(&self, expr: &Rc<Value>) -> Rc<Value> {
        /* Moves all if expressions to the top level of the expression */

        if let Some(conditional) = self.find_conditional(expr) {
            if let [_, condition, true_case, false_case] = conditional.to_list().unwrap().as_slice() {
                let expr_with_true_case = Value::substitute(expr, &conditional, true_case);
                let expr_with_false_case = Value::substitute(expr, &conditional, false_case);

                let true_case = self.move_conditionals_to_top_level(&expr_with_true_case);
                let false_case = self.move_conditionals_to_top_level(&expr_with_false_case);

                crate::refcount_list![
                    Value::Name("&if".into()).rc(),
                    condition.clone(),
                    true_case,
                    false_case
                ]
            } else {
                panic!("Liszp: expected syntax (if <cond> <true-case> <false-case>)");
            }
        } else {
            expr.clone()
        }
    }


    /* CPS conversion */

    fn convert_expr_with_continuation(expr: &Rc<Value>, continuation: &Rc<Value>) -> Rc<Value> {
        /* convert_expr() but with an explicit continuation for the entire expr */

        let mut converter = Self::new(continuation);
        let restructured = converter.move_conditionals_to_top_level(expr);

        if let Some(conditional) = converter.convert_conditional(expr) {
            conditional
        } else {
            converter.collect_components(&restructured);
            converter.assemble_cps_expression(expr)
        }
    }


    pub fn convert_conditional(&mut self, expr: &Rc<Value>) -> Option<Rc<Value>> {
        /* If expr is a conditional, this will convert it to CPS */

        let components = expr.to_list()?;

        if components.is_empty() || components[0].name() != "&if" {
            return None;
        } else if components.len() != 4 {
            panic!("Liszp: expected syntax (if <conditon> <true case> <false case>)");
        }

        let kwd_if = &components[0];
        let condition = &components[1];
        let true_case = &components[2];
        let false_case = &components[3];

        let conditional_expr = refcount_list![
            kwd_if.clone(),
            Value::Name("@@k-if".into()).rc(),
            Self::convert_expr_with_continuation(true_case, &self.continuation),
            Self::convert_expr_with_continuation(false_case, &self.continuation)
        ];

        let conditional_expr_continuation = refcount_list![
            Value::Name("&lambda".into()).rc(),
            Value::Name("@@k-if".into()).rc(),
            conditional_expr
        ];

        Some(Self::convert_expr_with_continuation(condition, &conditional_expr_continuation))
    }


    fn collect_components(&mut self, expr: &Rc<Value>) -> Rc<Value> {
       /* Collects the components of an expression via depth-first search
        *
        * Returns
        * -------
        * expr, but with its components replaced by numbered continuation
        * variables.
        */

        let components = match expr.to_list() {
            Some(xs) => {
                if xs.is_empty() {
                    return Value::Nil.rc()
                } else {
                    xs
                }
            },
            None => return expr.clone()
        };

        match components[0].name().as_str() {
            "&lambda" => Self::convert_lambda(&components),
            "&quote"  => self.convert_quote(expr),
            _ => {
                let mut component_labels = vec![ components[0].clone() ];

                // depth-first collection of sub-expressions
                for comp in components[1..].iter() {
                    component_labels.push(self.collect_components(comp));
                }

                self.dfs_expr_components.push(Value::cons_list(&component_labels));

                Value::Name(format!("@@k{}", self.dfs_expr_components.len() - 1)).rc()
            }
        }
    }


    pub fn convert_lambda(components: &Vec<Rc<Value>>) -> Rc<Value> {
        /* Converts a lambda expression to continuation-passing style */
    
        if let [kwd_lambda, args, body] = components.as_slice() {
            let lambda_continuation = Value::Name("@@k".into()).rc();

            let args = if let Value::Cons {..} = &**args {
                Value::cons(&lambda_continuation, args).rc()
            } else {
                refcount_list![ lambda_continuation.clone(), args.clone() ]
            };

            let body = Self::convert_expr_with_continuation(body, &lambda_continuation);

            refcount_list![
                kwd_lambda.clone(),
                args,
                body
            ]
        } else {
            panic!("Liszp: expected syntax (lambda <args> <body>");
        }
    }


    pub fn convert_quote(&mut self, expr: &Rc<Value>) -> Rc<Value> {
        /* Converts a quoted expression to continuation-passing style */

        self.dfs_expr_components.push(expr.clone());

        Value::Name(format!("@@k{}", self.dfs_expr_components.len() - 1)).rc()
    }


    pub fn assemble_cps_expression(&self, original_value: &Rc<Value>) -> Rc<Value> {
        /* Uses CPSConverter::dfs_expr_components to build a continuation-passing style expression */

        let mut converted_expression = self.continuation.clone();
        let mut atomic = true;

        // We start at the last expression to be evaluated and build the previous continuations
        // around it.
        for (continuation_number, expr) in self.dfs_expr_components.iter().enumerate().rev() {
            if let Value::Cons { car, cdr } = &**expr {
                let continuation = if atomic {
                    atomic = false;
                    converted_expression
                } else {
                    refcount_list![
                        Value::Name("&lambda".into()).rc(),
                        Value::Name(format!("@@k{}", continuation_number)).rc(),
                        converted_expression
                    ]
                };

                converted_expression = Value::cons(
                    car,
                    &Value::cons(&continuation, cdr).rc()
                ).rc();
            }
        }

        if atomic {
            refcount_list![
                converted_expression,
                original_value.clone()
            ]
        } else {
            converted_expression
        }
    }
}


pub fn convert_expr(expr: &Rc<Value>) -> Rc<Value> {
    /* Converts an expression to continuation-passing style */

    CPSConverter::convert_expr_with_continuation(expr, &Value::Name("no-continuation".into()).rc())
}