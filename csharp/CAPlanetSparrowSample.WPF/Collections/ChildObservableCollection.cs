using System.Collections.Generic;
using System.Collections.Specialized;
using System.ComponentModel;
using System.Linq;
using System.Reflection;

namespace System.Collections.ObjectModel;

public class ChildObservableCollection<T> : Collection<T>, INotifyCollectionChanged, INotifyPropertyChanged, ICollection
{
    private bool AllowDuplicat = false;
    public ChildObservableCollection() : this(false) { }
    public ChildObservableCollection(bool allowDuplicat)
    {
        AllowDuplicat = allowDuplicat;
    }
    public ChildObservableCollection(IEnumerable<T> collection, bool allowDuplicat = false) : this(allowDuplicat)
    {
        if (collection != null)
            Add([.. collection]);
    }

    public void Add(params T[] param) => AddRange(param);
    public void Add(IEnumerable<T> items) => AddRange(items);
    public void AddRange(IEnumerable<T> items)
    {
        if (AllowDuplicat == false)
        {
            var temp = items.Distinct().ToList();
            temp.RemoveAll(Contains);
            if (temp.Count == 0)
                return;
            items = temp;
        }
        items = [.. items];

        var startIndex = Count;
        var index = Count;

        foreach (var item in items)
        {
            base.InsertItem(index++, item);
            AddChildPropertyChanged(item);
        }

        OnCountPropertyChanged();
        OnIndexerPropertyChanged();
        OnCollectionChanged(NotifyCollectionChangedAction.Add, items.ToList(), startIndex);
    }
    public void Remve(params T[] param) => RemoveMany(param);
    public void Remove(IEnumerable<T> items) => RemoveMany(items);
    public void RemoveMany(IEnumerable<T> items)
    {
        items = [.. items];
        bool bRemove = false;
        foreach (var item in items)
        {
            int index = IndexOf(item);
            if (index < 0)
                continue;
            bRemove = true;
            base.RemoveItem(index);
            RemoveChildPropertyChanged(item);
        }
        if (bRemove == false)
            return;

        OnCountPropertyChanged();
        OnIndexerPropertyChanged();
        OnCollectionChanged(NotifyCollectionChangedAction.Remove, items.ToList());
    }
    public void Replace(IEnumerable<T> adds, IEnumerable<T> removes)
    {
        var removeTemp = removes.Distinct().ToList();
        removeTemp.RemoveAll(x => Contains(x) == false);
        removes = removeTemp;

        foreach (var item in removes)
        {
            var temp = IndexOf(item);
            if (temp < 0)
                continue;
            base.RemoveItem(temp);
            RemoveChildPropertyChanged(item);
        }

        if (AllowDuplicat == false)
        {
            var addTemp = adds.Distinct().ToList();
            addTemp.RemoveAll(Contains);
            adds = addTemp;
        }

        var startIndex = Count;
        var index = Count;

        foreach (var item in adds)
        {
            base.InsertItem(index++, item);
            AddChildPropertyChanged(item);
        }

        if (adds.Any() && removes.Any())
        {
            OnCountPropertyChanged();
            OnIndexerPropertyChanged();
            OnCollectionChanged(NotifyCollectionChangedAction.Replace, adds.ToList(), removes.ToList());
        }
        else if (adds.Any())
        {
            OnCountPropertyChanged();
            OnIndexerPropertyChanged();
            OnCollectionChanged(NotifyCollectionChangedAction.Add, adds.ToList(), startIndex);
        }
        else if (removes.Any())
        {
            OnCountPropertyChanged();
            OnIndexerPropertyChanged();
            OnCollectionChanged(NotifyCollectionChangedAction.Remove, removes.ToList());
        }
    }
    public void Move(int oldIndex, int newIndex)
    {
        T removedItem = this[oldIndex];

        base.RemoveItem(oldIndex);
        base.InsertItem(newIndex, removedItem);

        OnIndexerPropertyChanged();
        OnCollectionChanged(NotifyCollectionChangedAction.Move, removedItem, newIndex, oldIndex);
    }

    [Obfuscation(Feature = "all", Exclude = true)]
    event PropertyChangedEventHandler? INotifyPropertyChanged.PropertyChanged
    {
        add
        {
            PropertyChanged += value;
        }
        remove
        {
            PropertyChanged -= value;
        }
    }

    public event PropertyChangedEventHandler? PropertyChanged;
    public event NotifyCollectionChangedEventHandler? CollectionChanged;
    public event NotifyChildPropertyChangedEventHandler? ChildPropertyChanged;

    protected override void InsertItem(int index, T item)
    {
        if (AllowDuplicat == false)
        {
            if (Contains(item))
                return;
        }
        base.InsertItem(index, item);

        AddChildPropertyChanged(item);

        OnCountPropertyChanged();
        OnIndexerPropertyChanged();
        OnCollectionChanged(NotifyCollectionChangedAction.Add, item, index);
    }
    protected override void RemoveItem(int index)
    {
        T removedItem = this[index];

        base.RemoveItem(index);

        RemoveChildPropertyChanged(removedItem);

        OnCountPropertyChanged();
        OnIndexerPropertyChanged();
        OnCollectionChanged(NotifyCollectionChangedAction.Remove, removedItem, index);
    }
    protected override void SetItem(int index, T newItem)
    {
        T oldItem = this[index];
        base.SetItem(index, newItem);

        RemoveChildPropertyChanged(oldItem);
        AddChildPropertyChanged(newItem);

        OnIndexerPropertyChanged();
        OnCollectionChanged(NotifyCollectionChangedAction.Replace, newItem, oldItem, index);
    }
    protected override void ClearItems()
    {
        if (Count == 0)
            return;

        this.ForEach(RemoveChildPropertyChanged);

        base.ClearItems();
        OnCountPropertyChanged();
        OnIndexerPropertyChanged();
        OnCollectionReset();
    }

    private void OnPropertyChanged(PropertyChangedEventArgs e) => PropertyChanged?.Invoke(this, e);
    private void OnCollectionChanged(NotifyCollectionChangedEventArgs e) => CollectionChanged?.Invoke(this, e);
    private void OnChildPropertyChanged(NotifyChildPropertyChangedEventArgs e) => ChildPropertyChanged?.Invoke(this, e);

    private void OnCollectionChanged(NotifyCollectionChangedAction action, IList? item) => OnCollectionChanged(new NotifyCollectionChangedEventArgs(action, item));
    private void OnCollectionChanged(NotifyCollectionChangedAction action, IList? item, int startIndex) => OnCollectionChanged(new NotifyCollectionChangedEventArgs(action, item, startIndex));
    private void OnCollectionChanged(NotifyCollectionChangedAction action, IList newItems, IList oldItems) => OnCollectionChanged(new NotifyCollectionChangedEventArgs(action, newItems, oldItems));
    private void OnCollectionChanged(NotifyCollectionChangedAction action, object? item, int index) => OnCollectionChanged(new NotifyCollectionChangedEventArgs(action, item, index));
    private void OnCollectionChanged(NotifyCollectionChangedAction action, object? newItem, object? oldItem, int index) => OnCollectionChanged(new NotifyCollectionChangedEventArgs(action, newItem, oldItem, index));
    private void OnCollectionChanged(NotifyCollectionChangedAction action, object? item, int index, int oldIndex) => OnCollectionChanged(new NotifyCollectionChangedEventArgs(action, item, index, oldIndex));

    private void OnCountPropertyChanged() => OnPropertyChanged(EventArgsCache.CountPropertyChanged);
    private void OnIndexerPropertyChanged() => OnPropertyChanged(EventArgsCache.IndexerPropertyChanged);
    private void OnCollectionReset() => OnCollectionChanged(EventArgsCache.ResetCollectionChanged);

    private void OnChildPropertyChanged(object? sender, PropertyChangedEventArgs e)
    {
        if (sender is not T obj)
            return;

        int index = IndexOf(obj);
        if (index == -1)
            return;

        OnChildPropertyChanged(new NotifyChildPropertyChangedEventArgs(this, index, sender, e.PropertyName));
    }
    private void AddChildPropertyChanged(T item)
    {
        if (item is INotifyPropertyChanged propertyChanged)
            propertyChanged.PropertyChanged += OnChildPropertyChanged;
    }
    private void RemoveChildPropertyChanged(T item)
    {
        if (item is INotifyPropertyChanged propertyChanged)
            propertyChanged.PropertyChanged -= OnChildPropertyChanged;
    }

    internal static class EventArgsCache
    {
        internal static readonly PropertyChangedEventArgs CountPropertyChanged = new PropertyChangedEventArgs("Count");
        internal static readonly PropertyChangedEventArgs IndexerPropertyChanged = new PropertyChangedEventArgs("Item[]");
        internal static readonly NotifyCollectionChangedEventArgs ResetCollectionChanged = new NotifyCollectionChangedEventArgs(NotifyCollectionChangedAction.Reset);
    }
}
