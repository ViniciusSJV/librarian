import sys
p=sys.argv[1]+'/crates/librarian-ingest/src/resolve.rs'
s=open(p,encoding='utf8',newline='').read().replace('\r\n','\n')
def rep(a,b,n=1):
    global s
    assert s.count(a)==n,(s.count(a),a)
    s=s.replace(a,b)
rep("""    /// A constructor: the object is exactly of the class.
    exact: bool,
}""","""    /// A constructor: the object is exactly of the class.
    exact: bool,
    /// Step 2.4d-0: the receiver names the class itself.
    named: bool,
}""")
rep("parent: true, exact: false }","parent: true, exact: false, named: false }")
rep("PyReceiver { class, parent: false, exact: false }","PyReceiver { class, parent: false, exact: false, named: false }")
rep("        let class = self.python_class(caller, occurrence)?;\n        let constructed","""        let Some(class) = self.python_class(caller, occurrence) else {
            return self.receiver_class(caller, occurrence).map(|class| PyReceiver { class, parent: false, exact: true, named: true });
        };
        let constructed""")
rep("        Some(PyReceiver { class, parent: false, exact })","        Some(PyReceiver { class, parent: false, exact, named: false })")
rep("""    /// Step 2.4c (DEC-44): a `__new__`, a class decorator""","""    /// Step 2.4d-0: the class of the project that the written receiver names (`C`, `m.C`, `C.Nested`).
    fn receiver_class(&self, caller: &str, occurrence: &str) -> Option<&'g str> {
        let evidence = self.call_evidence.get(occurrence)?;
        let text = self.label(occurrence);
        let receiver = written_receiver(text)?;
        let segments: Vec<&str> = receiver.split('.').map(str::trim).collect();
        if !segments.iter().all(|s| !s.is_empty() && s.chars().all(|c| c.is_alphanumeric() || c == '_')) {
            return None;
        }
        let (at, path) = (evidence.start_byte, evidence.path.as_str());
        let (mut class, used) = (1..=segments.len()).find_map(|i| {
            let scoped = if i == 1 { self.python_name(path, at, segments[0], 0) } else { self.python_alias(path, at, &segments[..i].join("."), 0) };
            match scoped {
                Scoped::Target(t, _) if self.kind(t) == "class" => Some((t, i)),
                _ => None,
            }
        })?;
        for name in &segments[used..] {
            let nested: Vec<&'g str> = self.classes_named(name).into_iter().filter(|c| self.container(c) == Some(class)).collect();
            if nested.len() != 1 {
                return None;
            }
            class = nested[0];
        }
        // `C.__m` is mangled with the class that writes it.
        let name = call_parts(text)?.1;
        if name.starts_with("__") && !name.ends_with("__") && self.enclosing_method_class(caller).map(|(_, c)| c) != Some(class) {
            return None;
        }
        Some(class)
    }

    /// Step 2.4c (DEC-44): a `__new__`, a class decorator""")
rep("""        let class = receiver.class;
        match self.search(class, &name, receiver.parent) {
            Search::Found(found, at) => {
""","""        let class = receiver.class;
        match self.search(class, &name, receiver.parent) {
            Search::Found(_, at) if receiver.named && self.class_facts(at).names.contains(&name) => Chain::Unknown,
            Search::Undecided if receiver.named => Chain::Unknown,
            Search::Found(found, at) => {
""")
rep("for start in std::iter::once(receiver.class).chain(self.subclasses(receiver.class).iter().copied()) {","let below = if receiver.named { &[][..] } else { self.subclasses(receiver.class) };\n        for start in std::iter::once(receiver.class).chain(below.iter().copied()) {")
rep("None if !receiver.parent && self.baseless","None if !receiver.parent && !receiver.named && self.baseless")
rep("""                return Some("cls".to_owned());
            }""","""                return Some("cls".to_owned());
            }
            if self.receiver_class(caller, occurrence).is_some() {
                return Some("classe".to_owned());
            }""")
open(p,'w',encoding='utf8',newline='').write(s)
