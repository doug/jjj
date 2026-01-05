import * as vscode from 'vscode';
import { JJJ, Review } from '../jjj';

export class ReviewProvider implements vscode.TreeDataProvider<ReviewItem> {
  private _onDidChangeTreeData: vscode.EventEmitter<ReviewItem | undefined | null | void> =
    new vscode.EventEmitter<ReviewItem | undefined | null | void>();
  readonly onDidChangeTreeData: vscode.Event<ReviewItem | undefined | null | void> =
    this._onDidChangeTreeData.event;

  constructor(private jjj: JJJ) {}

  refresh(): void {
    this._onDidChangeTreeData.fire();
  }

  getTreeItem(element: ReviewItem): vscode.TreeItem {
    return element;
  }

  async getChildren(element?: ReviewItem): Promise<ReviewItem[]> {
    if (element) {
      return [];
    } else {
      try {
        const reviews = await this.jjj.listReviews();
        return reviews.map((review) => new ReviewItem(review));
      } catch (error) {
        vscode.window.showErrorMessage(`Failed to load reviews: ${error}`);
        return [];
      }
    }
  }
}

export class ReviewItem extends vscode.TreeItem {
  constructor(public readonly review: Review) {
    super(review.change_id.substring(0, 10), vscode.TreeItemCollapsibleState.None);
    this.tooltip = `${review.change_id} by ${review.author}`;
    this.description = `${review.status} [${review.comment_count} comments]`;

    this.command = {
      command: 'jjj.openReview',
      title: 'Open Review',
      arguments: [review],
    };

    this.iconPath = new vscode.ThemeIcon('comment-discussion');
  }
}
