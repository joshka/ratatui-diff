-- Decorate inline hex literals while preserving their selectable text and other output formats.
function Code(code)
  local hex = code.text:match('^#(%x+)$')
  if not FORMAT:match('html') or not hex then
    return nil
  end
  if #hex ~= 3 and #hex ~= 4 and #hex ~= 6 and #hex ~= 8 then
    return nil
  end
  return pandoc.Span({
    pandoc.RawInline('html', '<span class="color-swatch" aria-hidden="true" style="--swatch: #'
      .. hex .. '"></span>'),
    code,
  }, pandoc.Attr('', {'color-value'}))
end
