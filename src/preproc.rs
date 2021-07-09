/* The code in this file preprocesses Exprs:
 *
 * - formatting *all* names supplied in the source code to avoid
 *   collisions with non-user keywords (e.g. no-continuation).
 *
 * - macro expansion
 *
 * - CPS conversion.
 */

use crate::lexer::Expr;

use std::collections::LinkedList;

fn format_names(expr: &Expr) -> Expr {
    /* Appends '&' to all value names in the expression */

    match expr {
        Expr::Name { string, position } => {
            return Expr::Name {
                string: format!("{}&", string),
                position: position.clone()
            };
        },

        Expr::List { body, delim, position } => {
            return Expr::List {
                body: body.iter().map(format_names).collect(),
                delim: delim.clone(),
                position: position.clone()
            };
        },

        _ => return expr.clone()
    };
}

pub fn preproc_expressions(exprs: LinkedList<Expr>) -> LinkedList<Expr> {
   /* Fully preprocesses a list of expressions
    *
    * args
    * ----
    * - exprs: the list of expressions to be preprocessed
    *
    * returns
    * -------
    * a list of the same expressions, after:
    * 1. their names have been formatted
    * 2. all macros have been expanded (not yet implemented)
    * 3. all expressions have been converted to continuation passing style
    */

    let formatted = exprs.iter().map(format_names).collect();




    return formatted;
}
