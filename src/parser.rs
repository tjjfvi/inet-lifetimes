use crate::{order::Relation, types::*};

use std::{collections::HashMap, str::FromStr};

use highlight_error::highlight_error;
use TSPL::Parser as _;

struct Parser<'i> {
  input: &'i str,
  index: usize,
  type_lookup: HashMap<String, Type>,
  lt_lookup: HashMap<String, Lifetime>,
  agent_lookup: HashMap<String, Agent>,
  vars_lookup: HashMap<String, Var>,
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
    } else if self.peek_many(4) == Some("rule") {
      self.parse_rule_decl(ctx)
    } else if self.peek_many(3) == Some("net") {
      self.parse_net_decl(ctx)
    } else {
      self.expected("type, agent, or rule declaration")
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
    let agent = ctx.agents.push(AgentInfo { name: name.clone(), lt_ctx, ports });
    self.agent_lookup.insert(name, agent);
    Ok(())
  }

  fn parse_rule_decl(&mut self, ctx: &mut Ctx) -> Result<(), String> {
    self.consume("rule")?;
    self.vars_lookup.clear();
    let mut var_ctx = VarCtx::default();
    let a = self.parse_node(ctx, &mut var_ctx)?;
    let b = self.parse_node(ctx, &mut var_ctx)?;
    let result = self.parse_net(ctx, &mut var_ctx)?;
    ctx.rules.push(RuleInfo { var_ctx, a, b, result });
    Ok(())
  }

  fn parse_net_decl(&mut self, ctx: &mut Ctx) -> Result<(), String> {
    self.consume("net")?;
    let lt_ctx = self.parse_lt_ctx()?;
    self.vars_lookup.clear();
    let mut var_ctx = VarCtx::default();
    let (name, free_ports) = self.parse_node_like(|slf| {
      let var = slf.parse_var(&mut var_ctx)?;
      slf.consume(":")?;
      let label = slf.parse_port_label()?;
      Ok((var, label))
    })?;
    let nodes = self.parse_net(ctx, &mut var_ctx)?;
    ctx.nets.push(NetInfo { name, lt_ctx, var_ctx, free_ports, nodes });
    Ok(())
  }

  fn parse_node(&mut self, ctx: &mut Ctx, var_ctx: &mut VarCtx) -> Result<Node, String> {
    self.skip_trivia();
    let name_start = self.index;
    let (name, ports) = self.parse_node_like(|slf| slf.parse_var(var_ctx))?;
    let span = || highlight_error(name_start, name_start + name.len(), self.input);
    let Some(&agent) = self.agent_lookup.get(&name) else { Err(format!("unknown agent `{name}`:\n{}", span()))? };
    let expected_len = ctx.agents[agent].ports.len();
    if ports.len() != expected_len {
      Err(format!("expected {expected_len} ports, found {}:\n{}", ports.len(), span()))?
    }
    Ok(Node { agent, ports })
  }

  fn parse_net(&mut self, ctx: &mut Ctx, var_ctx: &mut VarCtx) -> Result<Vec<Node>, String> {
    self.consume("{")?;
    let mut nodes = vec![];
    while !self.try_consume("}") {
      nodes.push(self.parse_node(ctx, var_ctx)?);
    }
    Ok(nodes)
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
      let mut side = Side::External ^ self.try_consume("|");
      let mut prev = self.parse_lt_decl(&mut lt_ctx, side)?;
      loop {
        self.skip_trivia();
        let mut rel = match self.peek_one() {
          Some(',') => None,
          Some('<') => Some(Relation::LE),
          Some('>') => Some(Relation::GE),
          Some('|') => {
            if side == Side::External {
              side = Side::Internal;
              if self.try_consume("]") {
                break;
              }
              None
            } else {
              self.expected("comma or comparison operator")?
            }
          }
          Some(']') => break,
          _ => self.expected("comma, comparison operator, or separator")?,
        };
        self.advance_one();
        if let Some(rel) = &mut rel {
          if !self.try_consume("=") {
            *rel = rel.not_equal();
          }
        }
        let next = self.parse_lt_decl(&mut lt_ctx, side)?;
        if let Some(rel) = rel {
          match side {
            Side::External => lt_ctx.ex_order.relate(prev, next, rel),
            Side::Internal => lt_ctx.in_order.relate(prev, next, rel),
          }
        }
        prev = next;
      }
    }
    self.consume("]")?;
    Ok(lt_ctx)
  }

  fn parse_var(&mut self, var_ctx: &mut VarCtx) -> Result<Var, String> {
    let name = self.parse_name()?;
    Ok(
      *self
        .vars_lookup
        .entry(name)
        .or_insert_with_key(|name| var_ctx.vars.push(VarInfo { name: name.clone(), uses: vec![] })),
    )
  }

  fn parse_lt_decl(&mut self, lt_ctx: &mut LifetimeCtx, side: Side) -> Result<Lifetime, String> {
    let start = self.index;
    let name = self.parse_lt_name()?;
    let side = side ^ self.try_consume("?");
    let end = self.index;
    let lt = *self.lt_lookup.entry(name).or_insert_with_key(|name| lt_ctx.intro(format!("'{name}"), side));
    if lt_ctx.lifetimes[lt].side != side {
      Err(format!(
        "inconsistent known/unknown modifiers on lifetime `'{}`:\n{}",
        lt_ctx.lifetimes[lt].name,
        highlight_error(start, end, self.input()),
      ))?
    }
    Ok(lt)
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
      vars_lookup: Default::default(),
    }
    .parse_file()
  }
}
