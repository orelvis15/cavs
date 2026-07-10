using System;

namespace Cavs
{
    /// Thrown when a CAVS operation fails. `Code` is a stable `CAVS-E-*`
    /// identifier (e.g. CAVS-E-CANCELLED, CAVS-E-IO) so callers can branch
    /// on it without parsing the message.
    public class CavsException : Exception
    {
        public string Code { get; }

        public CavsException(string code, string message) : base(message)
        {
            Code = code;
        }
    }
}
