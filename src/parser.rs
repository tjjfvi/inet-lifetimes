use crate::types::*;

use std::{collections::HashMap, str::FromStr};

use highlight_error::highlight_error;
use TSPL::Parser as _;

struct Parser<'i> {
  input: &'i str,
  index: usize,
  type_lookup: HashMap<String, Type>,
  lt_lookup: HashMap<String, Lifetime>,
  agent_lookup: HashMap<String, Agent>,
}

impl<'i> TSPL::Parser<'i> for Parser<'i> {
  fn input(&mut self) -> &'i str {
    self.input
  }

  fn index(&mut self) -> &mut usize {
    &mut self.index
  }
}

impl<'i> Parser<'i> {
  fn parse_file(&mut self) -> Result<Ctx, String> {
    let mut ctx = Ctx::default();
    self.skip_trivia();
    while !self.is_eof() {
      self.parse_item(&mut ctx)?;
      self.skip_trivia();
    }
    Ok(ctx)
  }

  fn parse_item(&mut self, ctx: &mut Ctx) -> Result<(), String> {
    self.skip_trivia();
    if self.peek_many(4) == Some("type") {
      self.parse_type_decl(ctx)
    } else if self.peek_many(5) == Some("agent") {
      self.parse_agent_decl(ctx)
    } else {
      self.expected("type or agent decl")
    }
  }

  fn parse_type_decl(&mut self, ctx: &mut Ctx) -> Result<(), String> {
    self.consume("type")?;
    let name = self.parse_name()?;
    self.consume(":")?;
    self.skip_trivia();
    let polarity = match self.peek_one() {
      Some('+') => Pos,
      Some('-') => Neg,
      _ => self.expected("polarity")?,
    };
    self.advance_one();
    let not_info = TypeInfo { name: format!("!{name}") };
    let info = TypeInfo { name: name.clone() };
    let (pos_info, neg_info) = if polarity == Pos { (info, not_info) } else { (not_info, info) };
    let pos_ty = ctx.types.push(pos_info);
    let neg_ty = ctx.types.push(neg_info);
    let ty = if polarity == Pos { pos_ty } else { neg_ty };
    self.type_lookup.insert(name, ty);
    Ok(())
  }

  fn parse_agent_decl(&mut self, ctx: &mut Ctx) -> Result<(), String> {
    self.consume("agent")?;
    let lt_ctx = self.parse_lt_ctx()?;
    let (name, ports) = self.parse_node_like(Self::parse_port_label)?;
    let agent = Agent(ctx.agents.len());
    self.agent_lookup.insert(name.clone(), agent);
    ctx.agents.push(AgentInfo { name, lt_ctx, ports });
    Ok(())
  }

  fn parse_port_label(&mut self) -> Result<PortLabel, String> {
    Ok(PortLabel(self.parse_type()?, self.parse_lt()?))
  }

  fn parse_type(&mut self) -> Result<Type, String> {
    let inv = self.try_consume("!");
    let start = self.index;
    let name = self.parse_name()?;
    let end = self.index;
    let ty = self
      .type_lookup
      .get(&name)
      .copied()
      .ok_or_else(|| format!("unknown type `{name}`:\n{}", highlight_error(start, end, self.input())))?;
    Ok(if inv { !ty } else { ty })
  }

  fn parse_node_like<T>(
    &mut self,
    mut parse_elem: impl FnMut(&mut Self) -> Result<T, String>,
  ) -> Result<(String, Vec<T>), String> {
    let name = self.parse_name()?;
    self.consume("(")?;
    let mut elems = vec![parse_elem(self)?];
    while self.try_consume(",") {
      elems.push(parse_elem(self)?);
    }
    self.consume(")")?;
    Ok((name, elems))
  }

  fn parse_lt_ctx(&mut self) -> Result<LifetimeCtx, String> {
    self.lt_lookup.clear();
    let mut lt_ctx = LifetimeCtx::default();
    if !self.try_consume("[") {
      return Ok(lt_ctx);
    }
    if !self.try_consume("]") {
      let mut last = self.parse_lt_decl(&mut lt_ctx)?;
      loop {
        self.skip_trivia();
        let (is_related, is_less) = match self.peek_one() {
          Some(',') => (false, false),
          Some('>') => (true, false),
          Some('<') => (true, true),
          Some(']') => break,
          _ => self.expected("comma or comparison operator")?,
        };
        self.advance_one();
        let can_equal = self.try_consume("=");
        let next = self.parse_lt_decl(&mut lt_ctx)?;
        if is_related {
          let (a, b) = if is_less { (last, next) } else { (next, last) };
          lt_ctx.order.relate_lt(a, b, can_equal);
        }
        last = next;
      }
    }
    self.consume("]")?;
    Ok(lt_ctx)
  }

  fn parse_lt_decl(&mut self, lt_ctx: &mut LifetimeCtx) -> Result<Lifetime, String> {
    let name = self.parse_lt_name()?;
    Ok(*self.lt_lookup.entry(name).or_insert_with_key(|name| lt_ctx.intro(name.clone())))
  }

  fn parse_lt(&mut self) -> Result<Lifetime, String> {
    let start = self.index;
    let name = self.parse_lt_name()?;
    let end = self.index;
    Ok(
      self
        .lt_lookup
        .get(&name)
        .copied()
        .ok_or_else(|| format!("unknown lifetime `'{name}`:\n{}", highlight_error(start, end, self.input())))?,
    )
  }

  fn parse_lt_name(&mut self) -> Result<String, String> {
    self.consume("'")?;
    self.parse_name()
  }

  fn try_consume(&mut self, str: &str) -> bool {
    self.skip_trivia();
    if self.peek_many(str.len()) == Some(str) {
      self.advance_many(str.len());
      true
    } else {
      false
    }
  }
}

impl FromStr for Ctx {
  type Err = String;

  fn from_str(input: &str) -> Result<Self, Self::Err> {
    Parser {
      input,
      index: 0,
      type_lookup: Default::default(),
      lt_lookup: Default::default(),
      agent_lookup: Default::default(),
    }
    .parse_file()
  }
}
