use crate::{
  globals::{Component, ComponentInfo, Polarity, PortLabel, Type, TypeInfo},
  lifetimes::{Lifetime, LifetimeCtx, LifetimeInfo, Side},
  order::Relation,
  program::{AgentDef, NetDef, Node, Program, RuleDef, TypeDef},
  scope::ScopeBuilder,
  vars::{Var, VarCtx, VarInfo},
};

use std::str::FromStr;

use highlight_error::highlight_error;
use TSPL::Parser as _;

struct Parser<'i> {
  input: &'i str,
  index: usize,
  types: ScopeBuilder<'i, Type, TypeInfo>,
  components: ScopeBuilder<'i, Component, ComponentInfo>,
  lifetimes: ScopeBuilder<'i, Lifetime, LifetimeInfo>,
  vars: ScopeBuilder<'i, Var, VarInfo>,
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
  fn parse_file(&mut self) -> Result<Program, String> {
    let mut program = Program::default();
    self.skip_trivia();
    while !self.is_eof() {
      self.parse_item(&mut program)?;
      self.skip_trivia();
    }
    program.globals.types = self.types.finish();
    program.globals.components = self.components.finish();
    Ok(program)
  }

  fn parse_item(&mut self, program: &mut Program) -> Result<(), String> {
    self.skip_trivia();
    if self.peek_many(4) == Some("type") {
      program.types.push(self.parse_type_def()?)
    } else if self.peek_many(5) == Some("agent") {
      program.agents.push(self.parse_agent_def()?)
    } else if self.peek_many(4) == Some("rule") {
      program.rules.push(self.parse_rule_def()?)
    } else if self.peek_many(3) == Some("net") {
      program.nets.push(self.parse_net_def()?)
    } else {
      self.expected("type, agent, or rule declaration")?;
    }
    Ok(())
  }

  fn parse_type_def(&mut self) -> Result<TypeDef, String> {
    self.consume("type")?;
    let id = self.parse_type()?;
    self.consume(":")?;
    self.skip_trivia();
    let polarity = match self.peek_one() {
      Some('+') => Polarity::Pos,
      Some('-') => Polarity::Neg,
      _ => self.expected("polarity")?,
    };
    self.advance_one();
    Ok(TypeDef { id, polarity })
  }

  fn parse_agent_def(&mut self) -> Result<AgentDef, String> {
    self.consume("agent")?;
    let mut lt_ctx = self.parse_lt_ctx()?;
    let (id, ports) = self.parse_node_like(Self::parse_port_label)?;
    lt_ctx.lifetimes = self.lifetimes.finish();
    Ok(AgentDef { id, lt_ctx, ports })
  }

  fn parse_rule_def(&mut self) -> Result<RuleDef, String> {
    self.consume("rule")?;
    self.vars.ensure_empty();
    let a = self.parse_node()?;
    let b = self.parse_node()?;
    let result = self.parse_net()?;
    let var_ctx = VarCtx { vars: self.vars.finish() };
    Ok(RuleDef { var_ctx, a, b, result })
  }

  fn parse_net_def(&mut self) -> Result<NetDef, String> {
    self.consume("net")?;
    let mut lt_ctx = self.parse_lt_ctx()?;
    self.vars.ensure_empty();
    let (id, free_ports) = self.parse_node_like(|slf| {
      let var = slf.parse_var()?;
      slf.consume(":")?;
      let label = slf.parse_port_label()?;
      Ok((var, label))
    })?;
    let nodes = self.parse_net()?;
    let var_ctx = VarCtx { vars: self.vars.finish() };
    lt_ctx.lifetimes = self.lifetimes.finish();
    Ok(NetDef { id, lt_ctx, var_ctx, free_ports, nodes })
  }

  fn parse_node(&mut self) -> Result<Node, String> {
    let (component, ports) = self.parse_node_like(|slf| slf.parse_var())?;
    Ok(Node { component, ports })
  }

  fn parse_net(&mut self) -> Result<Vec<Node>, String> {
    self.consume("{")?;
    let mut nodes = vec![];
    while !self.try_consume("}") {
      nodes.push(self.parse_node()?);
    }
    Ok(nodes)
  }

  fn parse_port_label(&mut self) -> Result<PortLabel, String> {
    Ok(PortLabel(self.parse_type()?, self.parse_lt()?))
  }

  fn parse_type(&mut self) -> Result<Type, String> {
    let inv = self.try_consume("!");
    let name = self.parse_name()?;
    let ty = *self.types.lookup.entry(name).or_insert_with(|| {
      self.types.scope.push(format!("!{name}"), None);
      self.types.scope.push(name.to_owned(), None)
    });
    Ok(if inv { !ty } else { ty })
  }

  fn parse_node_like<T>(
    &mut self,
    mut parse_elem: impl FnMut(&mut Self) -> Result<T, String>,
  ) -> Result<(Component, Vec<T>), String> {
    let name = self.parse_name()?;
    let component = self.components.get(name);
    self.consume("(")?;
    let mut elems = Vec::new();
    while !self.try_consume(")") {
      elems.push(parse_elem(self)?);
      if !self.try_consume(",") {
        self.consume(")")?;
        break;
      }
    }
    Ok((component, elems))
  }

  fn parse_lt_ctx(&mut self) -> Result<LifetimeCtx, String> {
    self.lifetimes.ensure_empty();
    let mut lt_ctx = LifetimeCtx::default();
    if !self.try_consume("[") {
      return Ok(lt_ctx);
    }
    if !self.try_consume("]") {
      let mut side = Side::External ^ self.try_consume("|");
      let mut prev = self.parse_lt_decl(side)?;
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
        let next = self.parse_lt_decl(side)?;
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

  fn parse_var(&mut self) -> Result<Var, String> {
    let name = self.parse_name()?;
    Ok(*self.vars.lookup.entry(name).or_insert_with(|| self.vars.scope.push(name.to_owned(), Some(VarInfo::default()))))
  }

  fn parse_lt_decl(&mut self, side: Side) -> Result<Lifetime, String> {
    let start = self.index;
    let lt = self.parse_lt()?;
    let side = side ^ self.try_consume("?");
    let end = self.index;
    let info = self.lifetimes.scope.or_define(lt, || LifetimeInfo { side, min: None, max: None });
    if info.side != side {
      Err(format!(
        "inconsistent external/internal modifiers on lifetime `{}`:\n{}",
        self.lifetimes.scope.name(lt),
        highlight_error(start, end, self.input),
      ))?
    }
    Ok(lt)
  }

  fn parse_lt(&mut self) -> Result<Lifetime, String> {
    self.skip_trivia();
    let start = self.index;
    self.consume("'")?;
    self.take_while(Self::is_name_char);
    let name = &self.input[start..self.index];
    if name.len() <= 1 {
      self.expected("lifetime name")
    } else {
      Ok(self.lifetimes.get(name))
    }
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

  fn parse_name(&mut self) -> Result<&'i str, String> {
    self.skip_trivia();
    let name = self.take_while(Self::is_name_char);
    if name.is_empty() {
      self.expected("name")
    } else {
      Ok(name)
    }
  }

  fn is_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || "_.-/$".contains(c)
  }
}

impl FromStr for Program {
  type Err = String;

  fn from_str(input: &str) -> Result<Self, Self::Err> {
    Parser {
      input,
      index: 0,
      types: ScopeBuilder::default(),
      components: ScopeBuilder::default(),
      lifetimes: ScopeBuilder::default(),
      vars: ScopeBuilder::default(),
    }
    .parse_file()
  }
}
