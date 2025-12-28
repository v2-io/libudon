# TODO

- Refactor genmachine-rs and machine DSL:
  - example-- this whole block should all be done dynamically based on what it
    sees in the machine file so there's no extra genmachine work to do that is
    specific to the output. In the rare moments when there is, it should be
    done via liquid in the template.

```ruby
     1037      # Directive events
      1038      when 'directivestart'
      1039 -      "{ let name = self.term(); let span = self.span_from_mark(); self.emit(StreamingEvent::DirectiveStart { name, namespace: None, span }); }"
      1039 +      "{ let name = self.term(); let span = self.span_from_mark(); self.emit(StreamingEvent::DirectiveStart { name, raw: false, span }); }"
      1040 +    when 'directivestartraw'
      1041 +      "{ let name = self.term(); let span = self.span_from_mark(); self.emit(StreamingEvent::DirectiveStart { name, raw: true, span }); }"
      1042      when 'directiveend'
      1043        "self.emit(StreamingEvent::DirectiveEnd { span: Span::new(self.global_offset as usize, self.global_offset as usize) });"
      1044      when 'interpolation'
```
