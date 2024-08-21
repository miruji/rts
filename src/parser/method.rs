/*
    Method
*/

use crate::logger::*;
use crate::_exitCode;

use crate::tokenizer::line::*;
use crate::tokenizer::token::*;

use crate::parser::memoryCellList::*;
use crate::parser::memoryCell::*;

use crate::parser::readTokens;
use crate::parser::readLines;
use crate::parser::searchCondition;

use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use std::{io, io::Write};
use std::{borrow::Cow, str::SplitWhitespace};
use std::process::{Command, Output};
use std::thread::sleep;
use std::time::Duration;
use rand::Rng;

pub struct Method 
{
  pub           name: String,                        // unique name
                                                     // todo: Option
  pub          lines: Vec< Arc<RwLock<Line>> >,      // nesting lines
                                                     // todo: Option
  pub     parameters: Vec<Token>,                    // parameters
                                                     // todo: Option< Arc<RwLock<Token>> >
  pub         result: Option<Token>,                 // result type
      // if result type = None, => procedure
      // else => function
  pub memoryCellList: Arc<RwLock<MemoryCellList>>,   // todo: option< Arc<RwLock<MemoryCellList>> > ?
  pub        methods:    Vec< Arc<RwLock<Method>> >,
  pub         parent: Option< Arc<RwLock<Method>> >,
}
impl Method 
{
  pub fn new
  (
      name: String,
     lines: Vec< Arc<RwLock<Line>> >,
    parent: Option< Arc<RwLock<Method>> >,
  ) -> Self 
  {
    Method 
    {
                name,
               lines,
          parameters: Vec::new(),
              result: None,
      memoryCellList: Arc::new(RwLock::new(MemoryCellList::new())),
             methods: Vec::new(),
              parent
    }
  }

  // get method by name
  pub fn getMethodByName(&self, name: &str) -> Option<Arc<RwLock<Method>>> 
  {
    for childMethodLink in &self.methods 
    {
      let childMethod = childMethodLink.read().unwrap();
      if name == childMethod.name 
      {
        return Some(childMethodLink.clone());
      }
    }

    // check the parent method if it exists
    if let Some(parentLink) = &self.parent 
    {
      let parentMethod: RwLockReadGuard<'_, Method> = parentLink.read().unwrap();
      parentMethod.getMethodByName(name)
    } else { None }
  }

  // push memoryCell to self memoryCellList
  pub fn pushMemoryCell(&self, mut memoryCell: MemoryCell) -> ()
  {
    // basic
    if memoryCell.valueType != TokenType::Array 
    {
      memoryCell.value = self.memoryCellExpression(&mut memoryCell.value.tokens.clone());
    }
    // array
    let mut memoryCellList: RwLockWriteGuard<'_, MemoryCellList> = self.memoryCellList.write().unwrap();
    memoryCellList.value.push( Arc::new(RwLock::new(memoryCell)) );
  }

  // get memory cell by name
  pub fn getMemoryCellByName(&self, memoryCellName: &str) -> Option<Arc<RwLock<MemoryCell>>> 
  {
    // search in self
    if let Some(memoryCell) = getMemoryCellByName(self.memoryCellList.clone(), memoryCellName) 
    {
      return Some(memoryCell);
    }
    // search in parent
    if let Some(parentLink) = &self.parent 
    {
      let parent: RwLockReadGuard<'_, Method> = parentLink.read().unwrap();
      return parent.getMemoryCellByName(memoryCellName);
    }
    //
    None
  }

  // memory cell op
  pub fn memoryCellOp(&self, memoryCellLink: Arc<RwLock<MemoryCell>>, op: TokenType, opValue: Token) -> ()
  {
    if op != TokenType::Equals         &&
       op != TokenType::PlusEquals     && op != TokenType::MinusEquals &&
       op != TokenType::MultiplyEquals && op != TokenType::DivideEquals 
      { return; }

    // calculate new values
    let rightValue: Token = self.memoryCellExpression(&mut opValue.tokens.clone());
    let mut memoryCell = memoryCellLink.write().unwrap();
    // =
    if op == TokenType::Equals 
    {
      memoryCell.value = rightValue;
    } else 
    { // += -= *= /=
      let leftValue: Token = memoryCell.value.clone();
      if op == TokenType::PlusEquals     { memoryCell.value = calculate(&TokenType::Plus,     &leftValue, &rightValue); } else 
      if op == TokenType::MinusEquals    { memoryCell.value = calculate(&TokenType::Minus,    &leftValue, &rightValue); } else 
      if op == TokenType::MultiplyEquals { memoryCell.value = calculate(&TokenType::Multiply, &leftValue, &rightValue); } else 
      if op == TokenType::DivideEquals   { memoryCell.value = calculate(&TokenType::Divide,   &leftValue, &rightValue); }
    }
  }

  // update value
  fn replaceMemoryCellByName(&self, value: &mut Vec<Token>, length: &mut usize, index: usize) -> ()
  {
    if let Some(memoryCellLink) = self.getMemoryCellByName(&value[index].data) 
    {
      let memoryCell = memoryCellLink.read().unwrap();
      if index+1 < *length && value[index+1].dataType == TokenType::SquareBracketBegin 
      {
        let arrayIndex = // todo: rewrite if no UInt type ...
            self
                .memoryCellExpression(&mut value[index+1].tokens)
                .data.parse::<usize>();

        value.remove(index+1);
        *length -= 1;
        match arrayIndex 
        {
          Ok(idx) => 
          {
            value[index].data     = memoryCell.value.tokens[idx].data.clone();
            value[index].dataType = memoryCell.value.tokens[idx].dataType.clone();
          }
          Err(_) => 
          { // parsing errors
            value[index].data     = String::new();
            value[index].dataType = TokenType::None;
          }
        }
      } else 
      {
        value[index].data     = memoryCell.value.data.clone();
        value[index].dataType = memoryCell.value.dataType.clone();
      }
    } else 
    { // error -> skip
      value[index].data     = String::new();
      value[index].dataType = TokenType::None;
    }
  }

  // format quote
  fn formatQuote(&self, quote: String) -> String 
  {
    let mut result:           String    = String::new();
    let mut expressionBuffer: String    = String::new();
    let mut expressionRead:   bool      = false;
    let     chars:            Vec<char> = quote.chars().collect();

    let mut i:      usize = 0;
    let     length: usize = chars.len();
    let mut c:      char;

    while i < length 
    {
      c = chars[i];
      if c == '{' 
      {
        expressionRead = true;
      } else
      if c == '}' 
      {
        expressionRead = false;
        expressionBuffer += "\n";
        unsafe
        { 
          let expressionLineLink = &readTokens( expressionBuffer.as_bytes().to_vec(), false )[0];
          let expressionLine     = expressionLineLink.read().unwrap();
          let mut expressionBufferTokens: Vec<Token> = expressionLine.tokens.clone();
          result += &self.memoryCellExpression(&mut expressionBufferTokens).data;
        }
        expressionBuffer = String::new();
      } else 
      {
        if expressionRead 
        {
          expressionBuffer.push(c);
        } else 
        {
          result.push(c);
        }
      }
      i += 1;
    }
    result
  }

  // get expression parameters
  fn getExpressionParameters(&self, value: &mut Vec<Token>, i: usize) -> Vec<Token> 
  {
    let mut result = Vec::new();

    if let Some(tokens) = value.get(i+1).map(|v| &v.tokens) 
    { // get bracket tokens
      let mut expressionBuffer = Vec::new(); // buffer of current expression
      for (l, token) in tokens.iter().enumerate() 
      { // read tokens
        if token.dataType == TokenType::Comma || l+1 == tokens.len() 
        { // comma or line end
          expressionBuffer.push( token.clone() );
          result.push( self.memoryCellExpression(&mut expressionBuffer) );
          expressionBuffer.clear();
        } else 
        { // push new expression token
          expressionBuffer.push( token.clone() );
        }
      }
      value.remove(i+1); // remove bracket
    }

    result
  }

  // expression
  pub fn memoryCellExpression(&self, value: &mut Vec<Token>) -> Token 
  {
    let mut valueLength: usize = value.len();

    // 1 number
    if valueLength == 1 
    {
      if value[0].dataType != TokenType::CircleBracketBegin 
      {
        if value[0].dataType == TokenType::Word 
          { self.replaceMemoryCellByName(value, &mut valueLength, 0); } 
        else 
        if value[0].dataType == TokenType::FormattedRawString ||
           value[0].dataType == TokenType::FormattedString    ||
           value[0].dataType == TokenType::FormattedChar 
          { value[0].data = self.formatQuote(value[0].data.clone()); }
        return value[0].clone();
      }
    }

    //
    let mut i: usize = 0;
    let mut token: Token;
    // MemoryCell & function
    while i < valueLength 
    {
        if value[i].dataType == TokenType::Word 
        {
          // function
          if i+1 < valueLength && value[i+1].dataType == TokenType::CircleBracketBegin 
          {
            let functionName: String = value[i].data.clone();
            // todo: uint float ufloat ...
            if functionName == "int" 
            {
              // get expressions
              let expressions: Vec<Token> = self.getExpressionParameters(value, i);
              // 
              if expressions.len() > 0 
              {
                value[i]          = expressions[0].clone();
                value[i].dataType = TokenType::Int;
              } else 
              {
                value[i].data     = String::new();
                value[i].dataType = TokenType::None;
              }
              valueLength -= 1;
              continue;
            } else 
            if functionName == "char" 
            {
              // get expressions
              let expressions: Vec<Token> = self.getExpressionParameters(value, i);
              // 
              if expressions.len() > 0 
              {
                value[i]          = expressions[0].clone();
                value[i].data     = (value[i].data.parse::<u8>().unwrap() as char).to_string();
                value[i].dataType = TokenType::Char;
              } else 
              {
                value[i].data     = String::new();
                value[i].dataType = TokenType::None;
              }
              valueLength -= 1;
              continue;
            } else 
            if functionName == "str" 
            {
              // get expressions
              let expressions: Vec<Token> = self.getExpressionParameters(value, i);
              // 
              if expressions.len() > 0 
              {
                value[i]          = expressions[0].clone();
                value[i].dataType = TokenType::String;
              } else
              {
                value[i].data     = String::new();
                value[i].dataType = TokenType::None;
              }
              valueLength -= 1;
              continue;
            } else 
            if functionName == "type" 
            {
              // get expressions
              let expressions: Vec<Token> = self.getExpressionParameters(value, i);
              // 
              if expressions.len() > 0 
              {
                value[i].data = expressions[0].dataType.to_string();
                value[i].dataType = TokenType::String;
              } else 
              {
                value[i].data     = String::new();
                value[i].dataType = TokenType::None;
              }
              valueLength -= 1;
              continue;
            } else
            if functionName == "input" 
            {
              // get expressions
              let expressions: Vec<Token> = self.getExpressionParameters(value, i);
              //
              if expressions.len() > 0 
              {
                print!("{}",expressions[0].data);
                io::stdout().flush().unwrap(); // forced withdrawal of old
              }

              value[i].data = String::new();
              io::stdin().read_line(&mut value[i].data).expect("Input error"); // todo: delete error
              value[i].data = value[i].data.trim_end().to_string();
              value[i].dataType = TokenType::String;

              valueLength -= 1;
              continue;
            } else 
            if functionName == "randUInt" 
            {
              // get expressions
              let expressions: Vec<Token> = self.getExpressionParameters(value, i);
              // 
              if expressions.len() > 1 
              {
                let mut rng = rand::thread_rng();
                let min: usize = expressions[0].data.parse::<usize>().unwrap_or(0);
                let max: usize = expressions[1].data.parse::<usize>().unwrap_or(0);
                let randomNumber: usize = rng.gen_range(min..=max);

                value[i].data = randomNumber.to_string();
                value[i].dataType = TokenType::UInt;
              } else 
              {
                value[i].data     = String::new();
                value[i].dataType = TokenType::None;
              }

              valueLength -= 1;
              continue;
            } else 
            {
              let mut lineBuffer = Line::newEmpty();
              lineBuffer.tokens = value.clone();
              unsafe{ self.methodCall( Arc::new(RwLock::new(lineBuffer)) ); }

              // todo: rewrite
              if let Some(methodLink) = self.getMethodByName(&value[0].data) 
              {
                let method = methodLink.read().unwrap();
                if let Some(result) = &method.result 
                {
                  value[i].data     = result.data.clone();
                  value[i].dataType = result.dataType.clone();
                } else 
                {
                  value[i].data     = String::new();
                  value[i].dataType = TokenType::None;
                }
              } else 
              {
                value[i].data     = String::new();
                value[i].dataType = TokenType::None;
              }

              value.remove(i+1);
              valueLength -= 1;
              continue;
            }
          // array & basic cell
          } else 
          {
              self.replaceMemoryCellByName(value, &mut valueLength, i);
          }
        }

        if valueLength == 1 {
            break;
        }
        i += 1;
    }
    // bracket
    i = 0;
    while i < valueLength 
    {
      token = value[i].clone();
      if token.dataType == TokenType::CircleBracketBegin 
      {
        value[i] = self.memoryCellExpression(&mut token.tokens.clone());
      }
      i += 1;
    }
    // =
    i = 0;
    while i < valueLength 
    {
      if valueLength == 1 
      {
        break;
      }
      if i == 0 {
        i += 1;
        continue;
      }

      token = value[i].clone();
      if i+1 < valueLength && 
        (token.dataType == TokenType::Inclusion           || 
         token.dataType == TokenType::Joint               || 
         token.dataType == TokenType::Equals              || 
         token.dataType == TokenType::NotEquals           ||
         token.dataType == TokenType::GreaterThan         || 
         token.dataType == TokenType::LessThan            ||
         token.dataType == TokenType::GreaterThanOrEquals || 
         token.dataType == TokenType::LessThanOrEquals) {
        value[i-1] = calculate(&token.dataType, &value[i-1], &value[i+1]);
        
        value.remove(i); // remove op
        value.remove(i); // remove right value
        valueLength -= 2;
        continue;
      }

      i += 1;
    }
    // * and /
    i = 0;
    while i < valueLength 
    {
      if valueLength == 1 
      {
        break;
      }
      if i == 0 
      {
        i += 1;
        continue;
      }

      token = value[i].clone();
      if i+1 < valueLength && (token.dataType == TokenType::Multiply || token.dataType == TokenType::Divide) 
      {
        value[i-1] = calculate(&token.dataType, &value[i-1], &value[i+1]);

        value.remove(i); // remove op
        value.remove(i); // remove right value
        valueLength -= 2;
        continue;
      }

      i += 1;
    }
    // + and -
    i = 0;
    while i < valueLength 
    {
      if valueLength == 1 
      {
        break;
      }
      if i == 0 
      {
        i += 1;
        continue;
      }

      token = value[i].clone();
      // + and -
      if i+1 < valueLength && (token.dataType == TokenType::Plus || token.dataType == TokenType::Minus) 
      {
        value[i-1] = calculate(&token.dataType, &value[i-1], &value[i+1]);

        value.remove(i); // remove op
        value.remove(i); // remove right value
        valueLength -= 2;
        continue;
      } else
      // value -value2
      if token.dataType == TokenType::Int || token.dataType == TokenType::Float 
      {
        value[i-1] = calculate(&TokenType::Plus, &value[i-1], &value[i]);

        value.remove(i); // remove UInt
        valueLength -= 1;
        continue;
      }

      i += 1;
    }
    //
    if value.len() > 0 
    {
      value[0].clone()
    } else {
      Token::newEmpty(TokenType::None)
    }
  }

  /* search methods call
     e:
       methodCall(parameters)
  */
  pub unsafe fn methodCall(&self, lineLink: Arc<RwLock<Line>>) -> bool 
  {
    let line: RwLockReadGuard<'_, Line> = lineLink.read().unwrap();
    if line.tokens[0].dataType == TokenType::Word 
    {
      // add method call
      if line.tokens.len() > 1 && line.tokens[1].dataType == TokenType::CircleBracketBegin 
      {
        // check lower first char
        let token: &Token = &line.tokens[0];
        if token.data.starts_with(|c: char| c.is_lowercase()) 
        {
          let mut expressionValue: Vec<Token> = line.tokens[1].tokens.clone();
          // todo: multi-param
          // basic methods
          let mut result = true;
          {
            // go block up
            if token.data == "go" 
            {
              if let Some(parentLink) = &line.parent 
              {
                if let Some(methodParent) = &self.parent 
                {
                    // todo: check expressionValue
                    searchCondition(parentLink.clone(), methodParent.clone());
                }
              }
            } else
            // exit block up
            if token.data == "ex" 
            {
              println!("ex");
            } else
            // println
            if token.data == "println" 
            {
              println!("{}",formatPrint(
                &self.memoryCellExpression(&mut expressionValue).data
              ));
              io::stdout().flush().unwrap(); // forced withdrawal of old
            } else 
            // print
            if token.data == "print" {
              print!("{}",formatPrint(
                &self.memoryCellExpression(&mut expressionValue).data
              ));
              io::stdout().flush().unwrap(); // forced withdrawal of old
            } else 
            // sleep
            if token.data == "sleep" {
              let value = &self.memoryCellExpression(&mut expressionValue).data;
              let valueNumber = value.parse::<u64>().unwrap_or(0);
              sleep(Duration::from_millis(valueNumber));
            } else 
            // exec
            if token.data == "exec" {
              let expression: String              = self.memoryCellExpression(&mut expressionValue).data;
              let mut  parts: SplitWhitespace<'_> = expression.split_whitespace();

              let command: &str      = parts.next().expect("No command found in expression"); // todo: 
              let    args: Vec<&str> = parts.collect();

              let output: Output = 
                Command::new(command)
                  .args(&args)
                  .output()
                  .expect("Failed to execute process"); // todo: 
              let outputString: Cow<'_, str> = String::from_utf8_lossy(&output.stdout);
              if !outputString.is_empty() 
              {
                print!("{}", outputString);
              }
            } else 
            // exit
            if token.data == "exit" 
            {
              _exitCode = true;
            // custom method
            } else 
            {
              result = false;
            }
          }
          // custom methods
          if !result 
          {
            if let Some(calledMethodLink) = self.getMethodByName(&token.data) 
            {
              let mut   lineIndexBuffer: usize = 0;
              let mut linesLengthBuffer: usize = 
                {
                  let calledMethod = calledMethodLink.read().unwrap();
                  calledMethod.lines.len()
                };
              readLines(calledMethodLink, &mut lineIndexBuffer, &mut linesLengthBuffer);
              return true;
            }
          }
          return result;
        }
        //
      }
    }
    return false;
  }
}
