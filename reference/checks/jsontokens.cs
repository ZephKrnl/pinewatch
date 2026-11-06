// Runs the shipped Json.NET (Unturned_Data/Managed/Newtonsoft.Json.dll, 13.0.3) JsonTextReader over every case of a
// corpus and writes each token in the form crates/pinewatch-json/tests/corpus.rs compares.
//
// Corpus (args[0]): one JSON text per line, with ^ codes (^n LF, ^r CR, ^t tab, ^0 NUL, ^^ caret, ^uXXXX any UTF-16
// unit, ^E alone for the empty text); lines starting with # and empty lines are skipped.
// Output (args[1]): for each case "case N: <corpus line>", then one line per Read() call:
// "<returned> <TokenType>[ <value>] | depth D | path "P" | line L:C", or "error <type>: <message>" when Read()
// throws, which ends the case. Strings print with the same ^ codes. A date prints its ticks and kind; a Local date
// prints its UTC ticks, so the output doesn't depend on the machine's time zone.
using System;
using System.IO;
using System.Numerics;
using System.Text;
using Newtonsoft.Json;

static class JsonTokens
{
    static string Decode(string line)
    {
        if (line == "^E") return "";
        var sb = new StringBuilder();
        for (int i = 0; i < line.Length; i++)
        {
            char c = line[i];
            if (c != '^') { sb.Append(c); continue; }
            char code = line[++i];
            switch (code)
            {
                case 'n': sb.Append('\n'); break;
                case 'r': sb.Append('\r'); break;
                case 't': sb.Append('\t'); break;
                case '0': sb.Append('\0'); break;
                case '^': sb.Append('^'); break;
                case 'u': sb.Append((char) Convert.ToInt32(line.Substring(i + 1, 4), 16)); i += 4; break;
                default: throw new FormatException("bad code ^" + code);
            }
        }
        return sb.ToString();
    }

    static string Escape(string s)
    {
        var sb = new StringBuilder();
        foreach (char c in s)
        {
            if (c == '^') sb.Append("^^");
            else if (c >= 0x20 && c <= 0x7E) sb.Append(c);
            else sb.Append("^u").Append(((int) c).ToString("X4"));
        }
        return sb.ToString();
    }

    static string Value(object v)
    {
        if (v == null) return "";
        if (v is string) return " \"" + Escape((string) v) + "\"";
        if (v is long) return " I64 " + ((long) v).ToString(System.Globalization.CultureInfo.InvariantCulture);
        if (v is BigInteger)
            return " Big " + ((BigInteger) v).ToString(System.Globalization.CultureInfo.InvariantCulture);
        if (v is double) return " F64 " + BitConverter.DoubleToInt64Bits((double) v).ToString("x16");
        if (v is bool) return (bool) v ? " true" : " false";
        if (v is DateTime)
        {
            var d = (DateTime) v;
            long ticks = d.Kind == DateTimeKind.Local ? d.ToUniversalTime().Ticks : d.Ticks;
            return " Date " + ticks + " " + d.Kind;
        }
        return " ?" + v.GetType().FullName;
    }

    static void Main(string[] args)
    {
        Console.Error.WriteLine("corlib: " + typeof(object).Assembly.Location);
        Console.Error.WriteLine("Json.NET: " + typeof(JsonTextReader).Assembly.Location + " ("
            + typeof(JsonTextReader).Assembly.GetName().Version + ")");
        var output = new StringBuilder();
        int n = 0;
        foreach (string raw in File.ReadAllLines(args[0], Encoding.ASCII))
        {
            if (raw.Length == 0 || raw.StartsWith("#")) continue;
            n++;
            output.Append("case ").Append(n).Append(": ").Append(raw).Append('\n');
            var reader = new JsonTextReader(new StringReader(Decode(raw)));
            try
            {
                for (int i = 0; ; i++)
                {
                    if (i == 1000) { output.Append("limit\n"); break; }
                    bool more = reader.Read();
                    output.Append(more ? "true " : "false ").Append(reader.TokenType).Append(Value(reader.Value))
                        .Append(" | depth ").Append(reader.Depth).Append(" | path \"").Append(Escape(reader.Path))
                        .Append("\" | line ").Append(reader.LineNumber).Append(':').Append(reader.LinePosition)
                        .Append('\n');
                    if (!more) break;
                }
            }
            catch (Exception e)
            {
                output.Append("error ").Append(e.GetType().FullName).Append(": ").Append(Escape(e.Message))
                    .Append('\n');
            }
        }
        File.WriteAllText(args[1], output.ToString(), new UTF8Encoding(false));
        Console.Error.WriteLine(n + " cases");
    }
}
