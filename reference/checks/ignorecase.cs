// Checks that StringComparer.OrdinalIgnoreCase (DatDictionary's comparer) treats two one-unit strings as equal
// exactly when their ToUpperInvariant forms are equal, and hashes them alike, for every code unit against its
// upper case, lower case, ASCII case flip and neighbours. Prints the mismatch counts and a few probes.
using System;

static class IgnoreCase
{
    static void Main()
    {
        Console.WriteLine("corlib: " + typeof(object).Assembly.Location);
        StringComparer comparer = StringComparer.OrdinalIgnoreCase;
        int pairs = 0, equalityMismatches = 0, hashMismatches = 0;
        for (int i = 0; i <= 0xFFFF; i++)
        {
            char c = (char) i;
            foreach (int j in new[] { char.ToUpperInvariant(c), char.ToLowerInvariant(c), i ^ 0x20, i + 1, i - 1 })
            {
                if (j < 0 || j > 0xFFFF)
                    continue;
                char d = (char) j;
                string a = c.ToString(), b = d.ToString();
                bool expected = char.ToUpperInvariant(c) == char.ToUpperInvariant(d);
                pairs++;
                if (comparer.Equals(a, b) != expected)
                    equalityMismatches++;
                if (expected && comparer.GetHashCode(a) != comparer.GetHashCode(b))
                    hashMismatches++;
            }
        }
        Console.WriteLine("pairs " + pairs + ", equality mismatches " + equalityMismatches + ", hash mismatches " + hashMismatches);
        foreach (string[] probe in new[] {
            new[] { "Size", "ſize" }, new[] { "Item", "ıtem" }, new[] { "µ", "μ" },
            new[] { "ǅ", "Ǆ" }, new[] { "ᾀ", "ᾈ" }, new[] { "Size", "SIZE" } })
        {
            Console.WriteLine(probe[0] + " ~ " + probe[1] + ": " + comparer.Equals(probe[0], probe[1]));
        }
    }
}
