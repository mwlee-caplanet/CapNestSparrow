using System.Collections.Generic;
using System.Diagnostics.CodeAnalysis;

namespace System.Linq;

public static class LinqExtensions
{
    /// <summary>
    /// Immediately executes the given action on each element in the source sequence.
    /// </summary>
    /// <typeparam name="T">The type of the elements in the sequence</typeparam>
    /// <param name="source">The sequence of elements</param>
    /// <param name="action">The action to execute on each element</param>
    public static void ForEach<T>(this IEnumerable<T> source, Action<T> action)
    {
        ArgumentNullException.ThrowIfNull(source);
        ArgumentNullException.ThrowIfNull(action);

        foreach (var element in source)
            action(element);
    }
    public static IList<T> ReOrder<T>(this IList<T> source, Predicate<T> match)
    {
        ArgumentNullException.ThrowIfNull(source);
        ArgumentNullException.ThrowIfNull(match);
        var list = source.ToList();
        var startIndex = list.FindIndex(match);
        if (startIndex < 1)
            return source;

        return [.. list[startIndex..], .. list[..startIndex]];

    }
    public static IEnumerable<T> Behind<T>(this IList<T> source)
    {
        ArgumentNullException.ThrowIfNull(source);
        for (var i = source.Count - 1; i >= 0; --i)
            yield return source[i];
    }
    /// <summary>
    /// Immediately executes the given action on each element in the source sequence.
    /// Each element's index is used in the logic of the action.
    /// </summary>
    /// <typeparam name="T">The type of the elements in the sequence</typeparam>
    /// <param name="source">The sequence of elements</param>
    /// <param name="action">The action to execute on each element; the second parameter
    /// of the action represents the index of the source element.</param>
    public static void ForEach<T>(this IEnumerable<T> source, Action<T, int> action)
    {
        ArgumentNullException.ThrowIfNull(source);
        ArgumentNullException.ThrowIfNull(action);

        var index = 0;
        foreach (var element in source)
            action(element, index++);
    }

    /// <summary>
    /// Returns a sequence of <see cref="KeyValuePair{TKey,TValue}"/>
    /// where the key is the zero-based index of the value in the source
    /// sequence.
    /// </summary>
    /// <typeparam name="TSource">Type of elements in <paramref name="source"/> sequence.</typeparam>
    /// <param name="source">The source sequence.</param>
    /// <returns>A sequence of <see cref="KeyValuePair{TKey,TValue}"/>.</returns>
    /// <remarks>This operator uses deferred execution and streams its
    /// results.</remarks>
    public static IEnumerable<KeyValuePair<int, TSource>> Index<TSource>(this IEnumerable<TSource> source)
    {
        return source.Index(0);
    }

    /// <summary>
    /// Returns a sequence of <see cref="KeyValuePair{TKey,TValue}"/>
    /// where the key is the index of the value in the source sequence.
    /// An additional parameter specifies the starting index.
    /// </summary>
    /// <typeparam name="TSource">Type of elements in <paramref name="source"/> sequence.</typeparam>
    /// <param name="source">The source sequence.</param>
    /// <param name="startIndex"></param>
    /// <returns>A sequence of <see cref="KeyValuePair{TKey,TValue}"/>.</returns>
    /// <remarks>This operator uses deferred execution and streams its
    /// results.</remarks>
    public static IEnumerable<KeyValuePair<int, TSource>> Index<TSource>(this IEnumerable<TSource> source, int startIndex)
    {
        ArgumentNullException.ThrowIfNull(source);

        return source.Select((item, index) => new KeyValuePair<int, TSource>(startIndex + index, item));
    }

    public static bool IsEmpty<TSource>(this IEnumerable<TSource> source)
    {
        return !source.Any();
    }
    public static bool TryFirst<TSource>(this IEnumerable<TSource> source, Func<TSource, bool> predicate, [MaybeNullWhen(false)] out TSource value)
    {
        ArgumentNullException.ThrowIfNull(source);
        ArgumentNullException.ThrowIfNull(predicate);

        foreach (TSource element in source)
        {
            if (predicate(element))
            {
                value = element;
                return true;
            }
        }

        value = default;
        return false;
    }
}
