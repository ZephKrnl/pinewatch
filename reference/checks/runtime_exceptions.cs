// The exceptions the runtime itself raises (not managed constructors): type, exact Message (with its length) and
// the first line of ToString(), for each way the port can meet them.
using System;
using System.Collections.Generic;

static class RuntimeExceptions
{
    class Holder { public int field; public string text; }

    static void Probe(string name, Action action)
    {
        try
        {
            action();
            Console.WriteLine(name + "\tno exception");
        }
        catch (Exception e)
        {
            string first = e.ToString().Split('\n')[0].TrimEnd('\r');
            Console.WriteLine(name + "\t" + e.GetType().FullName + "\t[" + e.Message + "] (" + e.Message.Length + ")\t" + first);
        }
    }

    static int Zero() { return 0; }
    static int MinusOne() { return -1; }
    static object Null() { return null; }

    static void Main()
    {
        Console.WriteLine("corlib: " + typeof(object).Assembly.Location + ", pointer size " + IntPtr.Size);
        Probe("callvirt on null", () => Null().GetHashCode());
        Probe("string.Length on null", () => { string s = (string) Null(); Console.WriteLine(s.Length); });
        Probe("string method on null", () => { string s = (string) Null(); Console.WriteLine(s.ToLowerInvariant()); });
        Probe("field load on null", () => { Holder h = (Holder) Null(); Console.WriteLine(h.field); });
        Probe("field store on null", () => { Holder h = (Holder) Null(); h.text = "x"; });
        Probe("array length on null", () => { int[] a = (int[]) Null(); Console.WriteLine(a.Length); });
        Probe("array element on null", () => { int[] a = (int[]) Null(); Console.WriteLine(a[0]); });
        Probe("throw null", () => { throw (Exception) Null(); });
        Probe("unbox null", () => { Console.WriteLine((int) Null()); });
        Probe("null delegate", () => { Action a = (Action) Null(); a(); });
        Probe("interface call on null", () => { IDisposable d = (IDisposable) Null(); d.Dispose(); });
        Probe("foreach over null list", () => { List<int> l = (List<int>) Null(); foreach (int i in l) Console.WriteLine(i); });
        Probe("castclass", () => { object o = "x"; Console.WriteLine((Version) o); });
        Probe("unbox wrong type", () => { object o = 1L; Console.WriteLine((int) o); });
        Probe("array covariance store", () => { object[] a = new string[1]; a[0] = 1; });
        Probe("newarr -1", () => { Console.WriteLine(new byte[MinusOne()].Length); });
        Probe("newarr int.MinValue", () => { Console.WriteLine(new byte[int.MinValue + Zero()].Length); });
        Probe("newarr long -1", () => { long n = MinusOne(); Console.WriteLine(new byte[n].Length); });
        Probe("new int[-1, 1]", () => { Console.WriteLine(new int[MinusOne(), 1].Length); });
        Probe("checked add", () => { int x = int.MaxValue - Zero(); Console.WriteLine(checked(x + 1)); });
        Probe("checked (int)uint", () => { uint x = uint.MaxValue - (uint) Zero(); Console.WriteLine(checked((int) x)); });
        Probe("checked (byte)300", () => { int x = 300 + Zero(); Console.WriteLine(checked((byte) x)); });
        Probe("checked (int)double NaN", () => { double d = double.NaN; Console.WriteLine(checked((int) d)); });
        Probe("int.MinValue / -1", () => { int x = int.MinValue + Zero(); Console.WriteLine(x / MinusOne()); });
        Probe("int.MinValue % -1", () => { int x = int.MinValue + Zero(); Console.WriteLine(x % MinusOne()); });
        Probe("1 / 0", () => { Console.WriteLine(1 / Zero()); });
        Probe("1 % 0", () => { Console.WriteLine(1 % Zero()); });
        Probe("array index 2 of 1", () => { int[] a = new int[1]; Console.WriteLine(a[2 + Zero()]); });
        Probe("array index -1", () => { int[] a = new int[1]; Console.WriteLine(a[MinusOne()]); });
        Probe("string index 5 of 1", () => { string s = "x"; Console.WriteLine(s[5 + Zero()]); });
        Probe("string index -1", () => { string s = "x"; Console.WriteLine(s[MinusOne()]); });
        Probe("Array.CreateInstance -1", () => Array.CreateInstance(typeof(byte), MinusOne()));
        Probe("new List(-1)", () => Console.WriteLine(new List<byte>(MinusOne()).Count));
        Probe("Substring past end", () => Console.WriteLine("abc".Substring(4 + Zero())));
        Probe("stack overflow guard skipped", () => { });
    }
}
