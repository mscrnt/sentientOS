use anyhow::Result;
use super::{CorePackage, PackageCategory};

pub struct Calc;

impl CorePackage for Calc {
    fn name(&self) -> &'static str { "calc" }
    fn version(&self) -> &'static str { "1.0.0" }
    fn description(&self) -> &'static str { "Basic calculator for mathematical operations" }
    fn category(&self) -> PackageCategory { PackageCategory::Utils }
}

pub fn run(expression: &str) -> Result<String> {
    // Parse and evaluate the expression
    let trimmed = expression.trim();
    
    // Simple calculator - evaluate basic arithmetic
    // In a real implementation, we'd use a proper expression parser
    match evaluate_expression(trimmed) {
        Ok(result) => Ok(result.to_string()),
        Err(e) => Err(anyhow::anyhow!("Calculation error: {}", e)),
    }
}

fn evaluate_expression(expr: &str) -> Result<f64> {
    // Remove spaces
    let expr = expr.replace(" ", "");
    
    // For now, handle simple two-operand expressions
    if let Some(pos) = expr.find('+') {
        let left: f64 = expr[..pos].parse()?;
        let right: f64 = expr[pos+1..].parse()?;
        return Ok(left + right);
    }
    
    if let Some(pos) = expr.find('-') {
        let left: f64 = expr[..pos].parse()?;
        let right: f64 = expr[pos+1..].parse()?;
        return Ok(left - right);
    }
    
    if let Some(pos) = expr.find('*') {
        let left: f64 = expr[..pos].parse()?;
        let right: f64 = expr[pos+1..].parse()?;
        return Ok(left * right);
    }
    
    if let Some(pos) = expr.find('/') {
        let left: f64 = expr[..pos].parse()?;
        let right: f64 = expr[pos+1..].parse()?;
        if right == 0.0 {
            return Err(anyhow::anyhow!("Division by zero"));
        }
        return Ok(left / right);
    }
    
    // If no operator found, try to parse as a number
    expr.parse::<f64>()
        .map_err(|_| anyhow::anyhow!("Invalid expression"))
}