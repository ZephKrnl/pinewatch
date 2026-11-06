// Dumps, for every UTF-16 code unit, what the running class library's char functions return, and prints
// FNV-1a checksums in the form crates/pinewatch-mono/tests/chars.rs pins.
//
// Output file (args[0]): 65,536 records of 6 bytes: ToUpperInvariant (u16 LE), ToLowerInvariant (u16 LE),
// GetUnicodeCategory (u8), flags (u8: 1 IsWhiteSpace, 2 IsLetter, 4 IsDigit, 8 IsNumber, 16 IsUpper, 32 IsLower,
// 64 IsLetterOrDigit, 128 IsControl).
using System;
using System.Globalization;
using System.IO;

static class CharDump
{
    static ulong Fnv(Func<char, char> map)
    {
        ulong hash = 0xcbf29ce484222325UL;
        for (int c = 0; c <= 0xFFFF; c++)
        {
            ushort v = map((char) c);
            hash ^= (byte) (v & 0xFF);
            hash *= 0x100000001b3UL;
            hash ^= (byte) (v >> 8);
            hash *= 0x100000001b3UL;
        }
        return hash;
    }

    static void Main(string[] args)
    {
        Console.WriteLine("corlib: " + typeof(object).Assembly.Location + " (" + Environment.Version + ")");
        using (var output = new BinaryWriter(File.Create(args[0])))
        {
            int upperChanged = 0, lowerChanged = 0;
            for (int i = 0; i <= 0xFFFF; i++)
            {
                char c = (char) i;
                char upper = char.ToUpperInvariant(c);
                char lower = char.ToLowerInvariant(c);
                if (upper != c) upperChanged++;
                if (lower != c) lowerChanged++;
                int flags = (char.IsWhiteSpace(c) ? 1 : 0) | (char.IsLetter(c) ? 2 : 0) | (char.IsDigit(c) ? 4 : 0)
                    | (char.IsNumber(c) ? 8 : 0) | (char.IsUpper(c) ? 16 : 0) | (char.IsLower(c) ? 32 : 0)
                    | (char.IsLetterOrDigit(c) ? 64 : 0) | (char.IsControl(c) ? 128 : 0);
                output.Write((ushort) upper);
                output.Write((ushort) lower);
                output.Write((byte) char.GetUnicodeCategory(c));
                output.Write((byte) flags);
            }
            Console.WriteLine("ToUpperInvariant checksum 0x" + Fnv(char.ToUpperInvariant).ToString("x16") + ", changes " + upperChanged);
            Console.WriteLine("ToLowerInvariant checksum 0x" + Fnv(char.ToLowerInvariant).ToString("x16") + ", changes " + lowerChanged);
            TextInfo invariant = CultureInfo.InvariantCulture.TextInfo;
            Console.WriteLine("TextInfo.ToUpper checksum 0x" + Fnv(invariant.ToUpper).ToString("x16"));
            Console.WriteLine("TextInfo.ToLower checksum 0x" + Fnv(invariant.ToLower).ToString("x16"));
            Console.WriteLine("string.ToUpperInvariant checksum 0x" + Fnv(c => c.ToString().ToUpperInvariant()[0]).ToString("x16"));
            Console.WriteLine("CharUnicodeInfo.GetUnicodeCategory checksum 0x" + Fnv(c => (char) CharUnicodeInfo.GetUnicodeCategory(c)).ToString("x16"));
            Console.Write("IsWhiteSpace above U+00FF:");
            for (int i = 0x100; i <= 0xFFFF; i++)
                if (char.IsWhiteSpace((char) i))
                    Console.Write(" " + i.ToString("X4"));
            Console.WriteLine();
            Console.Write("IsWhiteSpace in Latin-1:");
            for (int i = 0; i < 0x100; i++)
                if (char.IsWhiteSpace((char) i))
                    Console.Write(" " + i.ToString("X2"));
            Console.WriteLine();
        }
    }
}
